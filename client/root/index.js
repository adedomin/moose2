import EXTENDED_COLORS from '/public/const/colors.js';
import { PIX_FMT_WIDTH, PIX_FMT_HEIGHT, MOOSE_SIZES, MOOSE_SIZE_DEFAULT_KEY, MOOSE_SIZE_HD_KEY } from '/public/const/sizes.js';
import { GridPaint } from '/public/gridpaint/index.js';

// constants
const EXTENDED_COLOR_START = 16;
const COLOR_ROW_LEN = 12;
const DEFAULT_COLOR = EXTENDED_COLORS.length - 1;
const START_PAL = 52;
const END_PAL = START_PAL + COLOR_ROW_LEN;
const BW_PAL = 88;
const BW_END = BW_PAL + COLOR_ROW_LEN;
// end constants

// html elements
const PAINTER_AREA = document.getElementById('painter-area');

const PENCIL = document.getElementById('pencil');
const LINE = document.getElementById('line');
const BUCKET = document.getElementById('bucket');
const TOOLS = [PENCIL, LINE, BUCKET];

const UNDO = document.getElementById('undo');
const REDO = document.getElementById('redo');
const HD = document.getElementById('hd');
const GRID = document.getElementById('grid');
const CLEAR = document.getElementById('clear');

const PALETTE = document.getElementById('painter-palette');
const PALETTE_SUB = document.getElementById('painter-palette-sub');

const NAME_INPUT = document.getElementById('name');
const SAVE = document.getElementById('save');

const MODAL_BACKDROP = document.getElementById('modal-backdrop');
const MODAL = document.getElementById('modal');
const MODAL_TITLE = document.getElementById('modal-title');
const MODAL_CONTENT = document.getElementById('modal-content');
const MODAL_CLOSE = document.getElementById('modal-close');

// end html elements


// state
let PAINTER = null;
let MOOSE_SIZE = MOOSE_SIZE_DEFAULT_KEY;
const DARK_THEME = window.matchMedia('(prefers-color-scheme: dark)');
// end state

// helpers
function isMobile() {
  return document.documentElement.clientWidth < 700;
}

function defaultLightness() {
  if (DARK_THEME.matches) {
    return 'light'
  } else {
    return 'dark'
  }
}

/** serialize the paining to base64 for moose api */
function serialize_painting_to_b64(painter = PAINTER) {
  let ret = "";
  for (let i = 0; i < painter.painting.length; ++i) {
    for (let j = 0; j < painter.painting[i].length; ++j) {
      ret += String.fromCharCode(painter.painting[i][j]);
    }
  }
  return btoa(ret);
}

function saveMoose() {
  return fetch("/new", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    credentials: "same-origin",
    body: JSON.stringify({
      name: NAME_INPUT.value,
      image: serialize_painting_to_b64(),
      dimensions: HD.classList.contains('selected') ? 'HD' : 'Default',
      created: (new Date()).toISOString(),
    }),
  });
}

function lightness(c) {
  const color = EXTENDED_COLORS[c];
  if (color.indexOf('#') !== 0 || color.length !== 9) return defaultLightness();
  // translucient, don't handle.
  if (color.slice(-2).toLowerCase() !== 'ff') return defaultLightness();
  const rgb = parseInt(color.slice(1, 7), 16);
  const r = rgb >> 16 ;
  const g = ( rgb >> 8 ) & 0xFF;
  const b = rgb & 0xFF;
  const lightness = ((r*299)+(g*587)+(b*114)) / 1000;
  return (lightness > 125) ? 'dark' : 'light';
}

// function removePainter(painter = PAINTER) {
//   if (painter !== null) {
//     painter.destroy();
//     if (painter.canvas) {
//       painter.canvas.parentNode.removeChild(painter.canvas);
//     }
//   }
// }

function addSelect(b, color) {
  b.classList.add('selected');
  b.classList.add(lightness(color));
}

function closeModal() {
  MODAL_BACKDROP.classList.add('close');
  MODAL.classList.add('close');
}

function openModal(title, content) {
  MODAL_TITLE.textContent = title;
  MODAL_CONTENT.textContent = content;
  MODAL.classList.remove('close');
  MODAL_BACKDROP.classList.remove('close');
}

function toggleHD() {
  if (HD.classList.toggle('selected')) {
    MOOSE_SIZE = MOOSE_SIZE_HD_KEY;
  } else {
    MOOSE_SIZE = MOOSE_SIZE_DEFAULT_KEY;
  }
}

function createPaletteBtn(color, sub = false) {
  const selector = sub === false ? '#painter-palette > .selected' : '#painter-palette-sub > .selected';
  const b = document.createElement('button');
  if (color !== DEFAULT_COLOR) {
    b.style.backgroundColor = EXTENDED_COLORS[color];
  } else {
    b.classList.add('transparent');
  }
  b.classList.add('palette-btn');
  b.innerText = '\xA0';
  b.title = 'switch to ' + color;
  if (PAINTER.colour == color) {
    addSelect(b, color);
  }
  b.addEventListener('click', () => {
    PAINTER.colour = color;
    document.querySelector(selector)?.classList.remove('selected');
    addSelect(b, color);
    if (color == DEFAULT_COLOR && !sub) {
        PALETTE_SUB.innerHTML = '';
        for (let i = BW_PAL; i < BW_END; ++i) {
          PALETTE_SUB.appendChild(createPaletteBtn(i, true));
        }
    } else if (!sub) {
        PALETTE_SUB.innerHTML = '';
        const row_off = (color - EXTENDED_COLOR_START) % COLOR_ROW_LEN;
        for (let i = 0; i < 6; ++i) {
          const subcol = (row_off + EXTENDED_COLOR_START) + (COLOR_ROW_LEN * i);
          PALETTE_SUB.appendChild(createPaletteBtn(subcol, true));
        }
    }
    if (!PAINTER.drawing) PAINTER.draw();
  })
  return b;
}

function init() {
  const [width, height] = MOOSE_SIZES.get(MOOSE_SIZE);
  const painter = new GridPaint({
    width,
    height,
    cellWidth: PIX_FMT_WIDTH,
    cellHeight: PIX_FMT_HEIGHT,
    outline: false,
    grid: true,
    palette: EXTENDED_COLORS,
    colour: DEFAULT_COLOR,
  });

  // removePainter();
  PAINTER_AREA.appendChild(painter.canvas);

  if (isMobile()) {
      setTimeout(() => state.painter.fitToWindow(), 0);
  }

  PAINTER = painter;

  TOOLS.forEach(el => {
    el.addEventListener('click', () => {
      PAINTER.setTool(el.id);
      TOOLS.forEach(el => {
        el.classList.remove('selected');
      })
      el.classList.add('selected');
      if (!PAINTER.drawing) PAINTER.draw();
    });
  });

  [UNDO, REDO].forEach(el => {
    el.addEventListener('click', () => {
      const oldh = PAINTER.height;
      const oldw = PAINTER.width;
      PAINTER.singleAction(el.id);
      if (oldh != PAINTER.height || oldw != PAINTER.width) {
        toggleHD();
      }
      if (!PAINTER.drawing) PAINTER.draw();
    });
  });

  HD.addEventListener('click', () => {
    toggleHD();
    const [width, height] = MOOSE_SIZES.get(MOOSE_SIZE);
    PAINTER.resizePainting(width, height, DEFAULT_COLOR);
    if (!PAINTER.drawing) PAINTER.draw();
  });

  GRID.addEventListener('click', () => {
    if (GRID.classList.toggle('selected')) {
      PAINTER.grid = true;
    } else {
      PAINTER.grid = false;
    }
    if (!PAINTER.drawing) PAINTER.draw();
  });

  CLEAR.addEventListener('click', () => {
    PAINTER.clearWith(DEFAULT_COLOR);
    if (!PAINTER.drawing) PAINTER.draw();
  });

  const dbtn = createPaletteBtn(DEFAULT_COLOR);
  PALETTE.appendChild(dbtn);
  for (let i = START_PAL; i < END_PAL; ++i) {
    const b = createPaletteBtn(i);
    PALETTE.appendChild(b);
  }
  dbtn.click();

  MODAL.addEventListener('click', e => {
    e.stopPropagation();
  });

  [MODAL_BACKDROP, MODAL_CLOSE].forEach(el => {
    el.addEventListener('click', () => {
      closeModal();
    });
  });

  SAVE.addEventListener('click', () => {
    let isOk = false;
    saveMoose().then(res => {
      isOk = res.ok;
      return res.json()
    }).then(body => {
      if (isOk) {
        openModal('Success', 'Moose Saved');
      } else {
        openModal('Error', body.msg);
      }
    }).catch(e => {
      openModal('Error', e.toString());
    }); 
  });

  PAINTER.attachHandlers();
  PAINTER.draw();

  document.addEventListener('keyup', e => {
    if (e.ctrlKey && e.key === 'z') {
      UNDO.click();
    } else if (e.ctrlKey && e.key === 'y') {
      REDO.click();
    } else if (e.key == 'Escape') {
      closeModal();
    }
  });
}
// end helpers

init()
