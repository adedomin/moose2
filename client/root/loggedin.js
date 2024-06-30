const login = document.getElementById('login');
fetch('/login/username', {
  method: 'POST',
  credentials: 'same-origin',
}).then(res => {
  return res.json();
}).then(username => {
  if (username !== null) {
    login.addEventListener('click', e => e.preventDefault());
    login.textContent = username;
  }
})
