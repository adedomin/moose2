import type { GridPaint as gp } from '../index.js';
/**
 * Resize the actual canvas and cell widths and heighs so they can fit to the parent window
 *
 * @see GridPaint#fitToWindow
 */
declare function resize(this: gp, w?: number, h?: number): void;
/**
 * Resize the drawing such that it has more cells to color.
 * The function will try and resize the painting such that it is centered.
 * NOTE: after calling this, you may need to call GridPaint#fitToWindow()
 *
 * @see GridPaint#fitToWindow
 */
declare function resizePainting(this: gp, w?: number, h?: number): void;
declare function fitToWindow(this: gp): void;
export { resize, resizePainting, fitToWindow };
