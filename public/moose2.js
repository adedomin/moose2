const search_form = document.getElementById('search-form');
const search_field = document.getElementById('search-field');
const moose_cards = document.getElementById('moose-cards');
const moose_card_template = document.getElementById('moose-card-template');
const error_banner = document.getElementById('hidden-banner-error');
const page_count = document.querySelector('.nav-block').dataset.pageCount;
const query_obj = new URLSearchParams(window.location.search);
const NO_MOOSE_ERR = "No Moose!";

function current_page() {
    window.location.pathname.slice('/gallery/'.length);
}

const page_cards = Array.from(moose_cards.querySelectorAll('.card'));

const blob_urls = [];
function del_old_search() {
    moose_cards.innerHTML = '';
    blob_urls.forEach(URL.revokeObjectURL);
    blob_urls.length = 0;
}

function restore_page() {
    moose_cards.append(...page_cards);
    if (page_cards.length > 0) {
        error_banner.classList.add('hidden');
    } else {
        error_banner.classList.remove('hidden');
    }
}

// UGH. src/moosedb.rs:37
// keep this up to date (probably won't change ever.)
//
const PIX_FMT_WIDTH = 16;
const PIX_FMT_HEIGHT = 24;
const dimensions = new Map([
    [26 * 15, [26, 15]],
    [36 * 22, [36, 22]],
]);

function RGBA(/** @type {Number} */ red,
              /** @type {Number} */ green,
              /** @type {Number} */ blue,
              /** @type {Number} */ trans) {
    return `rgba(${red}, ${green}, ${blue}, ${trans / 255})`;
}

// Starting to see why node.js and bundlers are popular...
// src/renderer.rs
const EXTENDED_COLORS = [
    // legacy mIRC colors
    RGBA(0xff, 0xff, 0xff, 0xff), // white
    RGBA(0x00, 0x00, 0x00, 0xff), // black
    RGBA(0x00, 0x00, 0x80, 0xff), // navy
    RGBA(0x00, 0x80, 0x00, 0xff), // green
    RGBA(0xff, 0x00, 0x00, 0xff), // red
    RGBA(0xa5, 0x2a, 0x2a, 0xff), // brown
    RGBA(0x80, 0x00, 0x80, 0xff), // purple
    RGBA(0x80, 0x80, 0x00, 0xff), // olive
    RGBA(0xff, 0xff, 0x00, 0xff), // yellow
    RGBA(0x00, 0xff, 0x00, 0xff), // lime
    RGBA(0x00, 0x80, 0x80, 0xff), // teal
    RGBA(0x00, 0xff, 0xff, 0xff), // cyan
    RGBA(0x00, 0x00, 0xff, 0xff), // blue
    RGBA(0xff, 0x00, 0xff, 0xff), // fuchsia
    RGBA(0x80, 0x80, 0x80, 0xff), // grey
    RGBA(0xd3, 0xd3, 0xd3, 0xff), // lightgrey
    // extended mIRC Colors
    // darkest
    RGBA(0x47, 0x00, 0x00, 0xff), // code 16 0
    RGBA(0x47, 0x21, 0x00, 0xff), // code 17 1
    RGBA(0x47, 0x47, 0x00, 0xff), // code 18 2
    RGBA(0x32, 0x47, 0x00, 0xff), // code 19 3
    RGBA(0x00, 0x47, 0x00, 0xff), // code 20 4
    RGBA(0x00, 0x47, 0x2c, 0xff), // code 21 5
    RGBA(0x00, 0x47, 0x47, 0xff), // code 22 6
    RGBA(0x00, 0x27, 0x47, 0xff), // code 23 7
    RGBA(0x00, 0x00, 0x47, 0xff), // code 24 8
    RGBA(0x2e, 0x00, 0x47, 0xff), // code 25 9
    RGBA(0x47, 0x00, 0x47, 0xff), // code 26 a
    RGBA(0x47, 0x00, 0x2a, 0xff), // code 27 b
    RGBA(0x74, 0x00, 0x00, 0xff), // code 28
    RGBA(0x74, 0x3a, 0x00, 0xff), // code 29
    RGBA(0x74, 0x74, 0x00, 0xff), // code 30
    RGBA(0x51, 0x74, 0x00, 0xff), // code 31
    RGBA(0x00, 0x74, 0x00, 0xff), // code 32
    RGBA(0x00, 0x74, 0x49, 0xff), // code 33
    RGBA(0x00, 0x74, 0x74, 0xff), // code 34
    RGBA(0x00, 0x40, 0x74, 0xff), // code 35
    RGBA(0x00, 0x00, 0x74, 0xff), // code 36
    RGBA(0x4b, 0x00, 0x74, 0xff), // code 37
    RGBA(0x74, 0x00, 0x74, 0xff), // code 38
    RGBA(0x74, 0x00, 0x45, 0xff), // code 39
    RGBA(0xb5, 0x00, 0x00, 0xff), // code 40
    RGBA(0xb5, 0x63, 0x00, 0xff), // code 41
    RGBA(0xb5, 0xb5, 0x00, 0xff), // code 42
    RGBA(0x7d, 0xb5, 0x00, 0xff), // code 43
    RGBA(0x00, 0xb5, 0x00, 0xff), // code 44
    RGBA(0x00, 0xb5, 0x71, 0xff), // code 45
    RGBA(0x00, 0xb5, 0xb5, 0xff), // code 46
    RGBA(0x00, 0x63, 0xb5, 0xff), // code 47
    RGBA(0x00, 0x00, 0xb5, 0xff), // code 48
    RGBA(0x75, 0x00, 0xb5, 0xff), // code 49
    RGBA(0xb5, 0x00, 0xb5, 0xff), // code 50
    RGBA(0xb5, 0x00, 0x6b, 0xff), // code 51 end of column
    RGBA(0xff, 0x00, 0x00, 0xff), // code 52
    RGBA(0xff, 0x8c, 0x00, 0xff), // code 53
    RGBA(0xff, 0xff, 0x00, 0xff), // code 54
    RGBA(0xb2, 0xff, 0x00, 0xff), // code 55
    RGBA(0x00, 0xff, 0x00, 0xff), // code 56
    RGBA(0x00, 0xff, 0xa0, 0xff), // code 57
    RGBA(0x00, 0xff, 0xff, 0xff), // code 58
    RGBA(0x00, 0x8c, 0xff, 0xff), // code 59
    RGBA(0x00, 0x00, 0xff, 0xff), // code 60
    RGBA(0xa5, 0x00, 0xff, 0xff), // code 61
    RGBA(0xff, 0x00, 0xff, 0xff), // code 62
    RGBA(0xff, 0x00, 0x98, 0xff), // code 63
    RGBA(0xff, 0x59, 0x59, 0xff), // code 64
    RGBA(0xff, 0xb4, 0x59, 0xff), // code 65
    RGBA(0xff, 0xff, 0x71, 0xff), // code 66
    RGBA(0xcf, 0xff, 0x60, 0xff), // code 67
    RGBA(0x6f, 0xff, 0x6f, 0xff), // code 68
    RGBA(0x65, 0xff, 0xc9, 0xff), // code 69
    RGBA(0x6d, 0xff, 0xff, 0xff), // code 70
    RGBA(0x59, 0xb4, 0xff, 0xff), // code 71
    RGBA(0x59, 0x59, 0xff, 0xff), // code 72
    RGBA(0xc4, 0x59, 0xff, 0xff), // code 73
    RGBA(0xff, 0x66, 0xff, 0xff), // code 74
    RGBA(0xff, 0x59, 0xbc, 0xff), // code 75
    // lightest
    RGBA(0xff, 0x9c, 0x9c, 0xff), // code 76
    RGBA(0xff, 0xd3, 0x9c, 0xff), // code 77
    RGBA(0xff, 0xff, 0x9c, 0xff), // code 78
    RGBA(0xe2, 0xff, 0x9c, 0xff), // code 79
    RGBA(0x9c, 0xff, 0x9c, 0xff), // code 80
    RGBA(0x9c, 0xff, 0xdb, 0xff), // code 81
    RGBA(0x9c, 0xff, 0xff, 0xff), // code 82
    RGBA(0x9c, 0xd3, 0xff, 0xff), // code 83
    RGBA(0x9c, 0x9c, 0xff, 0xff), // code 84
    RGBA(0xdc, 0x9c, 0xff, 0xff), // code 85
    RGBA(0xff, 0x9c, 0xff, 0xff), // code 86
    RGBA(0xff, 0x94, 0xd3, 0xff), // code 87
    RGBA(0x00, 0x00, 0x00, 0xff), // code 88 - blackest
    RGBA(0x13, 0x13, 0x13, 0xff), // code 89
    RGBA(0x28, 0x28, 0x28, 0xff), // code 90
    RGBA(0x36, 0x36, 0x36, 0xff), // code 91
    RGBA(0x4d, 0x4d, 0x4d, 0xff), // code 92
    RGBA(0x65, 0x65, 0x65, 0xff), // code 93
    RGBA(0x81, 0x81, 0x81, 0xff), // code 94
    RGBA(0x9f, 0x9f, 0x9f, 0xff), // code 95
    RGBA(0xbc, 0xbc, 0xbc, 0xff), // code 96
    RGBA(0xe2, 0xe2, 0xe2, 0xff), // code 97
    RGBA(0xff, 0xff, 0xff, 0xff), // code 98 - whitest
    RGBA(0x00, 0x00, 0x00, 0x00), // transparent (code 99)
];

function draw_moose(image) {
    const painting = atob(image);
    const [width, height] = dimensions.get(painting.length);
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
            draw_moose(moose.image).toBlob(blob => {
                const url = URL.createObjectURL(blob);
                blob_urls.push(url);
                img_link.src = url;
            });
            img_link_a.href = `/img/${encodeURIComponent(moose.name)}`;
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

search_form.addEventListener('submit', debounce_ev.bind(null, search, true));
search_field.addEventListener('input', debounce_ev.bind(null, search, false));
if (search_field.value != '') search();