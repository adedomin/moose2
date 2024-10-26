// Copyright (C) 2024  Anthony DeDominic
// SPDX-License-Identifier: GPL-3.0-or-later
const login = document.getElementById('login');
let loggedIn = false;

login.addEventListener('click', e => {
  if (loggedIn) {
    fetch('/logout', {
      method: 'POST',
      credentials: 'same-origin',
      headers: new Headers({'Content-Type': 'application/x-www-form-urlencoded'}),
    }).then(res => {
      if (res.ok) {
        loggedIn = false;
        login.textContent = 'Login';
      }
    }).catch(err => {
      login.textContent = 'LOGOUT FAILED (SEE CONSOLE)';
      console.error(err);
    });
    e.preventDefault();
  }
});

fetch('/login/username', {
  method: 'POST',
  credentials: 'same-origin',
}).then(res => {
  return res.json();
}).then(username => {
  if (username !== null) {
    login.textContent = username;
    loggedIn = true;
  }
}).catch(err => {
  console.error(err);
})
