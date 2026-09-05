// Copyright (C) 2026  Anthony DeDominic
// SPDX-License-Identifier: GPL-3.0-or-later
const login = document.getElementById('login');
const login_form = document.getElementById('log-inout-form');
const login_redir = document.getElementById('lio-redir');

const AUTHLVL_ANON = 0;

if (+login.dataset.authlevel > AUTHLVL_ANON) {
  const lev = e => {
    e.preventDefault();
    fetch('/logout', {
      method: 'POST',
      credentials: 'same-origin',
      headers: new Headers({'Content-Type': 'application/x-www-form-urlencoded'}),
    }).then(res => {
      if (res.ok) {
        login.value = 'Login';
        login_form.action = '/login';
        login_redir.value = window.location.pathname;
        login.removeEventListener('click', lev);
      }
    }).catch(err => {
      login.textContent = 'LOGOUT FAILED (SEE CONSOLE)';
      console.error(err);
    });
  };
  login.addEventListener('click', lev);
}
