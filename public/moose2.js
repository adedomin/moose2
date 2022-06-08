const search_form = document.getElementById('search-form');
const search_field = document.getElementById('search-field');
const moose_cards = document.getElementById('moose-cards');
const moose_card_template = document.getElementById('moose-card-template');
const error_banner = document.getElementById('hidden-banner-error');
const NO_MOOSE_ERR = "No Moose!";

const page_cards = Array.from(moose_cards.querySelectorAll('.card'));

function del_old_search() {
    moose_cards.innerHTML = '';
}

function restore_page() {
    moose_cards.append(...page_cards);
    if (page_cards.length > 0) {
        error_banner.classList.add('hidden');
    } else {
        error_banner.classList.remove('hidden');
    }
}

function build_cards(meese) {
    del_old_search();

    if (meese.length > 0) {
        error_banner.classList.add('hidden');
        for ([page, moose] of meese) {

            const template = moose_card_template.content.cloneNode(true);

            const card = template.querySelector('.card');
            const img_link_a = template.querySelector('a.nil');
            const img_link = template.querySelector('img.img');
            const text_node = template.querySelector('a.black-link');

            card.id = `-m-${encodeURIComponent(moose.name)}`;
            img_link.src = `/img/${encodeURIComponent(moose.name)}`;
            img_link_a.href = img_link.src;
            text_node.href = `/gallery/${page}#-m-${encodeURIComponent(moose.name)}`;
            text_node.textContent = moose.name;

            moose_cards.appendChild(card);
        }
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

function search() {
    let form = new URLSearchParams(new FormData(search_form));
    if (form.get('q') !== '') {
        fetch(`/search?${form.toString()}`).then(resp => {
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
    } else {
        del_old_search();
        restore_page();
    }
}

search_form.addEventListener('submit', debounce_ev.bind(null, search, true))
search_field.addEventListener('input', debounce_ev.bind(null, search, false));
if (search_field.value != '') search();