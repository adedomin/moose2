// Copyright (C) 2024  Anthony DeDominic
// SPDX-License-Identifier: GPL-3.0-or-later
import EXTENDED_COLORS from '/public/const/colors.js';
import { PIX_FMT_WIDTH, PIX_FMT_HEIGHT, MOOSE_SIZES } from '/public/const/sizes.js';

const search_form = document.getElementById('search-form');
const search_field = document.getElementById('search-field');
const moose_cards = document.getElementById('moose-cards');
const moose_card_template = document.getElementById('moose-card-template');
const error_banner = document.getElementById('hidden-banner-error');
const login = document.getElementById('login');
const login_form = document.getElementById('log-inout-form');
const login_redir = document.getElementById('lio-redir');

const NO_MOOSE_ERR = 'No Moose!';

function get_page_num(str) {
  return +(str.slice('/gallery/'.length));
}

function current_page() {
  return get_page_num(window.location.pathname);
}

let blob_urls = [];
function del_or_replace_cards(new_cards = undefined, new_blobs = undefined) {
  if (new_cards === undefined) {
    moose_cards.innerHTML = '';
  }
  else {
    moose_cards.replaceChildren(...new_cards);
  }
  blob_urls.forEach(URL.revokeObjectURL);
  blob_urls = new_blobs ?? [];
}

function draw_moose(image) {
  const painting = atob(image);
  const [width, height] = MOOSE_SIZES.get(painting.length);
  const c = document.createElement('canvas');
  c.width = width * PIX_FMT_WIDTH;
  c.height = height * PIX_FMT_HEIGHT;
  const ctx = c.getContext('2d');

  for (let idx = 0; idx < painting.length; idx++) {
    const color = painting.charCodeAt(idx);
    if (color === 99) continue;
    const y = Math.floor(idx / width) * PIX_FMT_HEIGHT;
    const x = (idx % width) * PIX_FMT_WIDTH;

    ctx.fillStyle = EXTENDED_COLORS[color];
    ctx.fillRect(x, y, PIX_FMT_WIDTH, PIX_FMT_HEIGHT);
  }

  return c;
}

function* page_renumber_range(to_page, page_count) {
  if (page_count < to_page) return;

  let start = to_page - 5;
  if (start < 0) {
    start += Math.abs(to_page - 5);
  }
  else if (start !== 0 && Math.abs(page_count - start) < 10) {
    start -= 10 - Math.abs(page_count - start);
  }

  for (let i = start; i < start + 10 && i < page_count; ++i) yield i;
}

function renumber_nav() {
  const to_page = current_page();
  const nav = document.querySelector('.nav-block');
  const page_count = +nav.children[nav.childElementCount - 2].dataset.page + 1;
  const page_range = [...page_renumber_range(to_page, page_count)];
  document.querySelectorAll('.nav-block').forEach(nav => {
    const left_arrow = nav.children[0];
    left_arrow.href = `/gallery/${to_page - 1}`;
    left_arrow.dataset.page = to_page - 1;

    const right_arrow = nav.children[nav.childElementCount - 1];
    right_arrow.href = `/gallery/${to_page + 1}`;
    right_arrow.dataset.page = to_page + 1;

    for (let i = 2; i < nav.childElementCount - 2; ++i) {
      nav.children[i].textContent = page_range[i - 2];
      nav.children[i].href = `/gallery/${page_range[i - 2]}`;
      nav.children[i].dataset.page = page_range[i - 2];
      if (page_range[i - 2] === to_page) {
        nav.children[i].classList.add('selected');
      }
      else {
        nav.children[i].classList.remove('selected');
      }
    }

    const first_page = nav.children[1];
    const last_page = nav.children[nav.childElementCount - 2];

    if (left_arrow.dataset.page < 0) {
      left_arrow.classList.add('disable');
      first_page.classList.add('disable');
    }
    else {
      left_arrow.classList.remove('disable');
      first_page.classList.remove('disable');
    }

    if (page_count - 1 < right_arrow.dataset.page) {
      right_arrow.classList.add('disable');
      last_page.classList.add('disable');
    }
    else {
      right_arrow.classList.remove('disable');
      last_page.classList.remove('disable');
    }
  });
}

function build_cards(meese_) {
  let meese = meese_;
  if (meese.length > 0) {
    error_banner.classList.add('hidden');
    if (meese[0].page === null || meese[0].page === undefined) {
      const curr = current_page();
      meese = meese.map(moose => ({ page: curr, moose }));
    }
    const new_els = [];
    const new_urls = [];
    const blob_promises = [];
    for (const { page, moose } of meese) {
      const template = moose_card_template.content.cloneNode(true);

      const card = template.querySelector('.card');
      const img_link_a = template.querySelector('a.nil');
      const img_link = template.querySelector('img.img');
      const text_node = template.querySelector('a.black-link');

      card.id = `-m-${encodeURIComponent(moose.name)}`;
      img_link_a.href = `/img/${encodeURIComponent(moose.name)}`;
      text_node.href = `/gallery/${page}#-m-${encodeURIComponent(moose.name)}`;
      text_node.textContent = moose.name;

      const canv = draw_moose(moose.image);
      img_link.height = canv.height;
      img_link.width = canv.width;
      blob_promises.push(new Promise(resolve => {
        canv.toBlob(blob => {
          const url = URL.createObjectURL(blob);
          img_link.src = url;
          new_urls.push(url);
          resolve();
        });
      }));

      new_els.push(card);
    }
    return Promise
      .all(blob_promises)
      .then(() => {
        del_or_replace_cards(new_els, new_urls);
      });
  }
  else {
    throw NO_MOOSE_ERR;
  }
}

const debounce_map = new Map();

function debounce_ev(func, bypass = false, event) {
  event.preventDefault();

  let timer = debounce_map.get(func);
  clearTimeout(timer);
  if (!bypass) {
    timer = setTimeout(() => func(), 200);
    debounce_map.set(func, timer);
  }
  else {
    func();
    debounce_map.delete(func);
  }
}

const PAGE = 0;
const SEARCH = 1;

function fetch_moose_arr(type, path) {
  loading(true);
  fetch(path).then(resp => {
    if (resp.ok) return resp.json();
    else throw Error(`Got non-OK status code: ${resp.status}`);
  }).then(meese => {
    switch (type) {
    case PAGE: return build_cards(meese);
    case SEARCH: return build_cards(meese.result);
    }
  }).then(() => {
    switch (type) {
    case PAGE: loading(false); break;
    // block pagination for search results.
    case SEARCH: loading(true); break;
    }
  })
  .catch(e => {
      del_or_replace_cards();
      error_banner.classList.remove('hidden');
      error_banner.textContent = e.toString();
    });
}

function add_nav_handlers() {
  document.querySelectorAll('.nav-block').forEach(nav => {
    for (let i = 0; i < nav.childElementCount; ++i) {
      const child = nav.children[i];
      child.dataset.page = get_page_num((new URL(child.href)).pathname);
      child.addEventListener('click', ev => {
        ev.preventDefault();
        // if (ev.currentTarget.parentElement.classList.has('disable')) return;
        if (+ev.currentTarget.dataset.page === current_page()) return;
        history.pushState(null, '', ev.currentTarget.href);
        login_redir.value = window.location.pathname;
        renumber_nav();
        search();
      });
    }
  });
}

function loading(is_loading) {
  document.querySelectorAll('.nav-block').forEach(nav => {
    if (is_loading) nav.classList.add('disable', 'loading');
    else nav.classList.remove('disable', 'loading');
  });
}

function search() {
  let form = new URLSearchParams(new FormData(search_form));
  form.delete('nojs');
  if (form.get('q') !== '') {
    history.replaceState(null, '', `${window.location.pathname}?${form.toString()}`);
    fetch_moose_arr(SEARCH, `/search?${form.toString()}`);
  }
  else {
    history.replaceState(null, '', window.location.pathname);
    fetch_moose_arr(PAGE, `/page/${current_page()}`);
  }
}

// HOLY FUCKING SHIT WHY?!?!??!?!
// clicking on hash links causes a popstate if the current state is pushed via history API
// who designed this shit?
let lasthash = '';

window.addEventListener('popstate', () => {
  if (lasthash === window.location.hash) {
    renumber_nav();
    search();
  }

  lasthash = window.location.hash;
});
search_form.addEventListener('submit', debounce_ev.bind(null, search, true));
search_field.addEventListener('input', debounce_ev.bind(null, search, false));

if (login.dataset.login === 'true') {
  const lev = e => {
    e.preventDefault();
    fetch('/logout', {
      method: 'POST',
      credentials: 'same-origin',
      headers: new Headers({'Content-Type': 'application/x-www-form-urlencoded'}),
    }).then(res => {
      if (res.ok) {
        login.value = 'Login';
        login_form.action = '/login';
        login_redir.value = window.location.pathname;
        login.removeEventListener('click', lev);
      }
    }).catch(err => {
      login.textContent = 'LOGOUT FAILED (SEE CONSOLE)';
      console.error(err);
    });
  };
  login.addEventListener('click', lev);
}

const query_obj = new URLSearchParams(window.location.search);
if (query_obj.has('q')) {
  const q = query_obj.get('q');
  search_field.value = q;
}
add_nav_handlers();
search();
