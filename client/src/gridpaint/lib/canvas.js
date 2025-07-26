// Copyright (C) 2016  Zorian Medwin
// Copyright (C) 2021  Anthony DeDominic
// See COPYING for License
import { isBrowser } from './browser.js';
let Canvas;
if (isBrowser) {
  Canvas = function (width, height) {
    const c = document.createElement('canvas');
    c.width = width || 300;
    c.height = height || 150;
    return c;
  };
}
else {
  Canvas = function (width, height) {
    // Cooerce the pureimage return to HTMLCanvasElement, in non-browser contexts
    // the actual HTML Canvas components are unused or will error anyhow.
    return {
      width: width || 300,
      height: height || 150,
      getContext: () => { },
    };
  };
}
export { Canvas };
