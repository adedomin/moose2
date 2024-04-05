// Copyright (C) 2016  Zorian Medwin
// Copyright (C) 2021  Anthony DeDominic
// See COPYING for License
function Canvas(width, height) {
    const c = document.createElement('canvas');
    c.width = width || 300;
    c.height = height || 150;
    return c;
};
export { Canvas };
