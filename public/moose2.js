import EXTENDED_COLORS from '/public/const/colors.js';
import { PIX_FMT_WIDTH, PIX_FMT_HEIGHT, MOOSE_SIZES } from '/public/const/sizes.js';

const search_form = document.getElementById('search-form');
const search_field = document.getElementById('search-field');
const moose_cards = document.getElementById('moose-cards');
const moose_card_template = document.getElementById('moose-card-template');
const error_banner = document.getElementById('hidden-banner-error');

const NO_MOOSE_ERR = "No Moose!";

function current_page() {
    return window.location.pathname.slice('/gallery/'.length);
}

const blob_urls = [];
function del_old_search() {
    moose_cards.innerHTML = '';
    blob_urls.forEach(URL.revokeObjectURL);
    blob_urls.length = 0;
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
        if (color == 99) continue;
        const y = Math.floor(idx / width) * PIX_FMT_HEIGHT;
        const x = (idx % width) * PIX_FMT_WIDTH;

        ctx.fillStyle = EXTENDED_COLORS[color];
        ctx.fillRect(x, y, PIX_FMT_WIDTH, PIX_FMT_HEIGHT);
    }

    return c;
}

function build_cards(meese_) {
    let meese = meese_;
    if (meese.length > 0) {
        error_banner.classList.add('hidden');
        if (!Array.isArray(meese[0])) {
            const curr = current_page();
            meese = meese.map(moose => [curr, moose]);
        }
        const new_els = [];
        for (const [page, moose] of meese) {
            const template = moose_card_template.content.cloneNode(true);

            const card = template.querySelector('.card');
            const img_link_a = template.querySelector('a.nil');
            const img_link = template.querySelector('img.img');
            const text_node = template.querySelector('a.black-link');

            card.id = `-m-${encodeURIComponent(moose.name)}`;
            draw_moose(moose.image).toBlob(blob => {
                const url = URL.createObjectURL(blob);
                blob_urls.push(url);
                img_link.src = url;
            });
            img_link_a.href = `/img/${encodeURIComponent(moose.name)}`;
            text_node.href = `/gallery/${page}#-m-${encodeURIComponent(moose.name)}`;
            text_node.textContent = moose.name;

            new_els.push(card);
        }
        del_old_search();
        moose_cards.append(...new_els);
    } else {
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
    } else {
        func()
        debounce_map.delete(func);
    }
}

function fetch_moose_arr(path) {
    fetch(path).then(resp => {
        if (resp.ok) return resp.json();
        else throw Error(`Got non-OK status code: ${resp.status}`);
    }).then(meese => {
        build_cards(meese);
    }).catch(e => {
        del_old_search();
        error_banner.classList.remove('hidden');
        error_banner.textContent = e.toString();
        console.error(e);
    })
}

function search() {
    let form = new URLSearchParams(new FormData(search_form));
    if (form.get('q') !== '') {
        history.replaceState(null, '', `${window.location.pathname}?${form.toString()}`);
        fetch_moose_arr(`/search?${form.toString()}`);
    } else {
        history.replaceState(null, '', `${window.location.pathname}`);
        fetch_moose_arr(`/page/${current_page()}`);
    }
}

search_form.addEventListener('submit', debounce_ev.bind(null, search, true));
search_field.addEventListener('input', debounce_ev.bind(null, search, false));
// if (search_field.value != '') search();
const query_obj = new URLSearchParams(window.location.search);
if (query_obj.has('q')) {
    const q = query_obj.get('q');
    search_field.value = q;
}
search();