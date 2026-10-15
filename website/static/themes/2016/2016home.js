(function () {
  const $ = (id) => document.getElementById(id);
  const esc = (s) => String(s == null ? '' : s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const num = (n) => Number(n || 0).toLocaleString('en-US');
  const json = (u) => fetch(u, { credentials: 'same-origin' }).then((r) => (r.ok ? r.json() : Promise.reject(r.status)));

  $('t16-user-search').addEventListener('submit', (e) => {
    e.preventDefault(); const v = $('user-search').value.trim(); if (v) location.href = '/users/' + encodeURIComponent(v) + '?mode=osu';
  });
  $('t16-map-search').addEventListener('submit', (e) => {
    e.preventDefault(); const v = $('beatmap-search').value.trim(); if (v) location.href = '/beatmapsets?q=' + encodeURIComponent(v);
  });

  function login(session) {
    const box = $('t16-login');
    if (session.user) {
      box.innerHTML = '<span class="nav-login">Welcome, <b><a href="/users/' + encodeURIComponent(session.user) + '">' + esc(session.user) + '</a></b></span> ' +
        '<a href="/users/' + encodeURIComponent(session.user) + '"><i class="icon-user" style="color:#808080"></i> Profile</a> ' +
        '<a href="/profile#login"><i class="icon-exchange" style="color:#808080"></i> Switch profile</a>';
    } else {
      box.innerHTML = '<span class="nav-login">Welcome, guest!</span> <a href="/profile#login"><i class="icon-caret-down"></i> Login</a>';
    }
  }

  function mostPlayed(p) {
    const body = $('t16-most-body'), list = (p && p.most_played) || [];
    $('t16-most-sub').textContent = p ? 'Played by ' + p.name : '';
    if (!list.length) { body.innerHTML = '<tr class="row-dark"><td colspan="3">' + (p ? 'No plays recorded yet.' : 'Sign in to see your most played beatmaps.') + '</td></tr>'; return; }
    body.innerHTML = list.slice(0, 5).map((m, i) => {
      const b = m.beatmap || {}; const set = Number(b.beatmapset_id) || 0;
      const link = set ? 'href="/beatmapsets/' + set + '"' : '';
      return '<tr class="' + (i % 2 ? 'row-light' : 'row-dark') + '"><td>' + num(m.count) + '</td><td>' +
        (set ? '<img src="https://b.ppy.sh/thumb/' + set + '.jpg" loading="lazy" alt="" width="80" height="60" onerror="this.style.visibility=\'hidden\'">' : '') +
        '<a ' + link + '>' + esc((b.artist || 'Unknown artist') + ' - ' + (b.title || 'Unknown beatmap')) + '</a></td><td>' + esc(b.creator || '') + '</td></tr>';
    }).join('');
  }

  json('/site/session').then((s) => {
    login(s);
    const n = (s.profiles || []).length;
    const stats = (plays, name) => '<b>' + num(n) + '</b> local profile' + (n === 1 ? '' : 's') + ', <b>' + (s.active ? 1 : 0) + '</b> online now.<br>A total of <b>' + num(plays) + '</b> plays by ' + esc(name) + '!';
    $('t16-stats').innerHTML = stats(0, s.user || 'this player');
    if (!s.user) { mostPlayed(null); return; }
    return json('/site/profile?name=' + encodeURIComponent(s.user) + '&mode=osu').then((p) => {
      $('t16-stats').innerHTML = stats(p.playcount, p.name);
      mostPlayed(p);
    });
  }).catch(() => { $('t16-most-body').innerHTML = '<tr class="row-dark"><td colspan="3">Could not load data.</td></tr>'; });
})();
