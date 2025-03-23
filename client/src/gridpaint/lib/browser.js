// Copyright (C) 2021  Anthony DeDominic
// SPDX-License-Identifier: LGPL-3.0-or-later
let isBrowser = false;
if (typeof process === 'undefined') {
  isBrowser = true;
}
else if (process?.title === 'browser') {
  isBrowser = true;
}
export { isBrowser };
