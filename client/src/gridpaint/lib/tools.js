// Copyright (C) 2016  Zorian Medwin
// Copyright (C) 2021  Anthony DeDominic
// See COPYING for License
import { bucket } from './bucket.js';
import { clear, clearWith } from './clear.js';
import { replace } from './replace.js';
import { line, line_approx } from './line.js';
const MAX_HISTORY = 64;
function clone_painting(painting) {
  return Array.from(painting, el => el.slice());
}
function pushHistory(top, bottom) {
  if (top.length === 0) {
    return;
  }
  const top_canvas = top.pop();
  if (top_canvas == null) {
    return;
  }
  const oldh = top_canvas.length;
  const oldw = top_canvas[0].length;
  bottom.push(this.painting.splice(0, this.painting.length));
  this.painting = top_canvas;
  if (oldh !== this.height) {
    this.height = oldh;
    this.canvas.height = this.height * this.cellHeight;
  }
  if (oldw !== this.width) {
    this.width = oldw;
    this.canvas.width = this.width * this.cellWidth;
  }
}
// activated when the user's finger or mouse is pressed
function apply(isApplied) {
  if (isApplied !== undefined) {
    this.isApplied = isApplied;
  }
  else {
    this.isApplied = !this.isApplied;
  }
  // activate the tool for initial mouse press
  if (this.isApplied) {
    this.action();
  }
}
/** compared oldPainting to painting & push the changes to history
 * @param state any object that returns on undo/redo.
 */
function compare() {
  if (this.oldPainting.length === this.painting.length) {
    if (this.painting.every((el, i) => el.toString() === this.oldPainting[i].toString())) {
      return;
    }
  }
  this.undoHistory.push(clone_painting(this.oldPainting));
  this.undoHistory.splice(0, this.undoHistory.length - MAX_HISTORY);
  this.redoHistory.length = 0;
}
// fill in grid units one by one
function pencil() {
  const x = this.cursor.x;
  const y = this.cursor.y;
  if (x >= 0 && x < this.width && y >= 0 && y < this.height) {
    this.painting[y][x] = this.colour;
  }
}
// redo the last painting action performed (if any)
function redo() {
  pushHistory.apply(this, [
    this.redoHistory,
    this.undoHistory,
  ]);
}
// undo the last painting action performed (if any)
function undo() {
  pushHistory.apply(this, [
    this.undoHistory,
    this.redoHistory,
  ]);
}
export {
/* Action Tools */
  pencil, bucket, line,
  /* Single Click Action Tools */
  redo, undo, clear, clearWith,
  /* These do not have "action()'s" */
  apply, replace, compare, line_approx };
