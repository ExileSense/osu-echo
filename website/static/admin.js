(function () {
  'use strict';
  var $ = function (id) { return document.getElementById(id); };
  var esc = function (s) { return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; }); };

  function api(path, body) {
    var opt = { credentials: 'same-origin', headers: { 'X-Admin': '1' } };
    if (body !== undefined) { opt.method = 'POST'; opt.headers['Content-Type'] = 'application/json'; opt.body = JSON.stringify(body); }
    return fetch('/site/admin/' + path, opt).then(function (r) {
      return r.json().catch(function () { return {}; }).then(function (d) { if (!r.ok) throw new Error(d.error || ('Request failed (' + r.status + ')')); return d; });
    });
  }
  var toastTimer;
  function toast(msg, bad) {
    var t = $('adm-toast'); t.textContent = msg; t.className = 'adm-toast' + (bad ? ' is-err' : ''); t.hidden = false;
    clearTimeout(toastTimer); toastTimer = setTimeout(function () { t.hidden = true; }, 3500);
  }
  function fail(e) { toast(e.message || String(e), true); }
  function ago(ts) {
    var s = Math.max(0, Math.floor(Date.now() / 1000 - ts));
    if (s < 60) return s + 's ago'; if (s < 3600) return Math.floor(s / 60) + ' min ago';
    if (s < 86400) return Math.floor(s / 3600) + ' h ago'; return Math.floor(s / 86400) + ' d ago';
  }
  function dur(s) {
    var d = Math.floor(s / 86400), h = Math.floor(s % 86400 / 3600), m = Math.floor(s % 3600 / 60), x = s % 60, o = [];
    if (d) o.push(d + 'd'); if (d || h) o.push(h + 'h'); o.push(m + 'm'); o.push(x + 's'); return o.join(' ');
  }
  function flagEmoji(cc) {
    cc = String(cc || '').toUpperCase(); if (!/^[A-Z]{2}$/.test(cc)) return '';
    return String.fromCodePoint(0x1f1e6 + cc.charCodeAt(0) - 65, 0x1f1e6 + cc.charCodeAt(1) - 65);
  }
  var MODES = ['osu!', 'taiko', 'catch', 'mania'];
  function modal(html) { var m = $('adm-modal'); $('adm-modal-card').innerHTML = html; m.hidden = false; return $('adm-modal-card'); }
  function closeModal() { $('adm-modal').hidden = true; }
  $('adm-modal').addEventListener('mousedown', function (e) { if (e.target === this) closeModal(); });
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape') closeModal(); });

  var lastTest = null;
  function applyTest() {
    document.querySelectorAll('[data-kt]').forEach(function (el) {
      var r = lastTest && lastTest[el.dataset.kt];
      if (!r) { el.textContent = ''; el.className = 'adm-kt'; return; }
      el.className = 'adm-kt is-' + r.state;
      el.textContent = (r.state === 'ok' ? '✓ ' : r.state === 'fail' ? '✗ ' : r.state === 'error' ? '! ' : '') + r.message;
    });
    sgRender();
  }
  function runTest() { return api('keys/test', {}).then(function (d) { lastTest = d; applyTest(); return d; }); }
  function refreshDataset(btn, idle, done) {
    btn.disabled = true; btn.textContent = 'Updating…';
    api('ladder/refresh', {}).then(function (d) { SG.dataset = true; toast('Dataset updated (data up to ' + d.last_date + ').'); if (done) done(); sgRender(); })
      .catch(fail).then(function () { btn.disabled = false; btn.textContent = idle; });
  }

  var SG = { active: false, step: 1, users: 0, dataset: false, user: null, windows: false };
  var STEP_HASH = { 1: 'keys', 2: 'other', 3: 'users' };
  function sgLoad() {
    return api('setup').then(function (d) { SG.active = d.active; SG.step = d.step; SG.users = d.users; SG.windows = d.windows; SG.hostsReady = d.hosts_ready !== false; sgNavigate(); sgRender(); if (d.active && d.step === 1 && !lastTest) runTest().catch(function () {}); }).catch(function () {});
  }
  function sgNavigate() {
    if (!SG.active) return;
    var h = STEP_HASH[SG.step];
    if (h && location.hash.slice(1) !== h) location.hash = h; else if (h) show(h);
  }
  function sgGo(step) {
    SG.step = step;
    return api('setup', { step: step }).then(function (d) { SG.users = d.users; sgNavigate(); sgRender(); if (step === 1 && !lastTest) runTest().catch(function () {}); }).catch(fail);
  }
  function sgAccountDone() {
    api('setup').then(function (d) { SG.users = d.users; if (SG.active && SG.step === 3 && d.users > 0) sgGo(4); else sgRender(); }).catch(function () {});
  }
  function sgSkipAll() {
    var c = modal('<h2>Skip the setup guide?</h2><div class="adm-hint">You can still change everything from the sections on the left. The guide will not come back.</div>' +
      '<div class="adm-actions"><button class="adm-btn adm-btn--primary" id="sk-ok" type="button">Skip guide</button><button class="adm-btn" id="sk-no" type="button">Keep going</button></div>');
    c.querySelector('#sk-no').onclick = closeModal;
    c.querySelector('#sk-ok').onclick = function () { api('setup', { action: 'finish' }).then(function () { closeModal(); SG.active = false; sgRender(); }).catch(fail); };
  }
  function sgGoProfile() {
    var b = $('sg-go'); if (b) b.disabled = true;
    var csrf;
    api('setup', { action: 'finish' }).then(function () { return fetch('/site/session', { credentials: 'same-origin' }).then(function (r) { return r.json(); }); }).then(function (s) {
      csrf = s.csrf;
      if (s.user) return;
      return (SG.user ? Promise.resolve(SG.user) : api('users').then(function (d) { return d.users.length ? d.users[0].name : null; })).then(function (name) {
        if (!name) return;
        return site('/site/login', { username: name }, csrf);
      });
    }).then(function () { location.href = '/profile'; }).catch(function (e) { fail(e); if (b) b.disabled = false; });
  }
  function sgRender() {
    var el = $('sg');
    if (!SG.active) { el.hidden = true; el.innerHTML = ''; document.body.classList.remove('has-sg'); return; }
    var st = SG.step, body = '', next = '', ok;
    var hostsWarn = (!SG.hostsReady && SG.windows) ? '<p style="color:#ffd37a">osu.localhost is not set up yet (hosts file). Close osu-echo and start it as administrator, then reload this page with CTRL + F5.</p>' : '';
    var dots = '<div class="sg__steps">' + [1, 2, 3, 4].map(function (n) { return '<i class="' + (n <= st ? 'is-done' : '') + '"></i>'; }).join('') + '</div>';
    if (st === 1) {
      var t = lastTest || {}, rows = [['osu! API v1', 'v1'], ['osu! API v2 (OAuth)', 'v2'], ['osu!daily API', 'daily']];
      var all = rows.every(function (r) { return t[r[1]] && t[r[1]].state === 'ok'; });
      body = '<p>Welcome! Put in your keys on the page. The links under each box tell you where to get them.</p><ul class="sg__list">' +
        rows.map(function (r) {
          var x = t[r[1]], s = x ? x.state : 'unset', sym = s === 'ok' ? '✓' : s === 'fail' ? '✗' : s === 'error' ? '!' : '–';
          return '<li><span class="sg__dot is-' + s + '">' + sym + '</span>' + r[0] + (x ? ' <span class="adm-muted">· ' + esc(x.message) + '</span>' : '') + '</li>';
        }).join('') + '</ul>';
      if (all) body += '<p>All keys work. Now update the rank dataset instead of using the one included with the program:</p><button class="adm-btn adm-btn--primary" id="sg-ds" type="button">' + (SG.dataset ? '✓ Dataset updated — update again' : 'Update dataset') + '</button>';
      else body += '<button class="adm-btn" id="sg-test" type="button">Test keys</button>';
      ok = all && SG.dataset;
      next = '<button class="adm-btn adm-btn--primary sg__btn" id="sg-next" type="button"' + (ok ? '' : ' disabled') + '>----&gt;</button>';
    } else if (st === 2) {
      body = '<p>Choose your preferred <b>default site theme</b> in the <b>Other</b> tab. Visitors can still pick their own in the footer.</p><p class="adm-muted">The other settings on that page (leaderboard size, folders, Discord…) can be changed any time.</p>';
      next = '<button class="adm-btn adm-btn--primary sg__btn" id="sg-next" type="button">----&gt;</button>';
    } else if (st === 3) {
      body = '<p>Create your account by choosing a username in <b>Manage Users</b>, or import your profile from osu.ppy.sh.</p>' + (SG.users > 0 ? '<p class="adm-muted">There ' + (SG.users === 1 ? 'is 1 account' : 'are ' + SG.users + ' accounts') + ' already.</p>' : '');
      next = '<button class="adm-btn adm-btn--primary sg__btn" id="sg-next" type="button"' + (SG.users > 0 ? '' : ' disabled') + '>----&gt;</button>';
    } else {
      body = '<p>To connect, please put <code>-devserver localhost</code> in your file properties.</p><p>You can access this page by going to <code>osu.localhost/admin</code>.</p>';
      next = '<button class="adm-btn adm-btn--primary sg__btn" id="sg-go" type="button">Go</button>';
    }
    el.innerHTML = '<div class="sg__head">Setup guide <small>Step ' + st + ' of 4</small></div>' + dots + '<div class="sg__body">' + hostsWarn + body + '</div>' +
      '<div class="sg__foot"><button class="sg__skip" id="sg-skip" type="button">' + (st === 4 ? '' : (st === 1 || st === 2 ? 'skip step' : 'skip setup')) + '</button>' + next + '</div>';
    el.hidden = false; document.body.classList.add('has-sg');
    var q = function (id) { return el.querySelector('#' + id); };
    if (q('sg-next')) q('sg-next').onclick = function () {
      if (st === 2) { var th = $('ot-theme').value; (th ? api('other/theme', { theme: th }) : Promise.resolve()).then(function () { return sgGo(3); }).catch(fail); }
      else sgGo(st + 1);
    };
    if (q('sg-skip')) q('sg-skip').onclick = function () { if (st === 3) sgSkipAll(); else sgGo(st + 1); };
    if (q('sg-go')) q('sg-go').onclick = sgGoProfile;
    if (q('sg-ds')) q('sg-ds').onclick = function () { refreshDataset(this, SG.dataset ? '✓ Dataset updated — update again' : 'Update dataset'); };
    if (q('sg-test')) q('sg-test').onclick = function () { var b = this; b.disabled = true; b.textContent = 'Testing…'; runTest().catch(fail).then(function () { b.disabled = false; b.textContent = 'Test keys'; }); };
  }

  function site(path, body, csrf) {
    var o = { credentials: 'same-origin' };
    if (body !== undefined) { o.method = 'POST'; o.headers = { 'Content-Type': 'application/json', 'X-CSRF-Token': csrf }; o.body = JSON.stringify(body); }
    return fetch(path, o).then(function (r) {
      return r.json().catch(function () { throw new Error('The server returned an unexpected response.'); }).then(function (d) { if (!r.ok) throw new Error(d.error || 'The request failed.'); return d; });
    });
  }

  var impDone = null, impBusy = false;
  function impStatus(msg, cls) { var s = $('imp-status'); s.hidden = !msg; s.textContent = msg; s.className = 'adm-imp__status' + (cls ? ' is-' + cls : ''); }
  function impOpen(done) {
    impDone = done; impStatus('');
    $('imp').hidden = false; $('imp-query').focus();
  }
  function impClose() { if (impBusy) return; $('imp').hidden = true; }
  function impRun() {
    var query = $('imp-query').value.trim();
    if (!query) { impStatus('Enter an official osu! username or user ID.', 'err'); return; }
    var name = $('imp-name').value.trim() || (/^\d+$/.test(query) ? '' : query);
    if (!name) { impStatus('Enter a local account name (the official lookup was a numeric ID).', 'err'); return; }
    var mode = $('imp-mode').value, limit = +$('imp-limit').value, csrf, profileMsg = '';
    impBusy = true; $('imp-go').disabled = true; $('imp-cancel').disabled = true;
    impStatus('Step 1/3: creating the account and fetching the profile from osu.ppy.sh…');
    var wantProfile = $('imp-bio').checked || $('imp-details').checked || $('imp-avatar').checked || $('imp-cover').checked;
    fetch('/site/session', { credentials: 'same-origin' }).then(function (r) { return r.json(); }).then(function (s) {
      csrf = s.csrf; return site('/site/login', { username: name }, csrf);
    }).then(function () {
      if (!wantProfile) return null;
      return site('/site/import_official', { query: query, apply: false, import_avatar: $('imp-avatar').checked, import_cover: $('imp-cover').checked, import_bio: $('imp-bio').checked, import_details: $('imp-details').checked }, csrf).then(function (res) {
        var body = { username: name, reset_avatar: false, reset_cover: false };
        if ($('imp-bio').checked || $('imp-details').checked) body.details = res.details || {};
        if (res.avatar_data_url) body.avatar = res.avatar_data_url;
        if (res.cover_data_url) body.cover = res.cover_data_url;
        profileMsg = 'Profile of ' + res.official_username + ' (#' + res.official_id + ') imported';
        return site('/site/settings', body, csrf);
      });
    }).then(function () {
      impStatus((profileMsg ? profileMsg + '. ' : '') + 'Step 2/3: downloading scores from osu.ppy.sh (paced, can take up to a minute)…');
      return site('/site/import_scores', { query: query, mode: mode, limit: limit }, csrf);
    }).then(function (res) {
      var returned = (res.fetched || []).reduce(function (n, f) { return n + f.returned; }, 0);
      if (!returned) throw new Error((profileMsg ? profileMsg + '. ' : '') + 'osu.ppy.sh returned no top scores for ' + res.official_name + '. Check the mode, or whether that account has ranked scores.');
      var info = (profileMsg ? profileMsg + ' · ' : '') + 'Imported ' + res.imported + ' score' + (res.imported === 1 ? '' : 's');
      if (!$('imp-ranks').checked) return info + ' · rank lookup skipped';
      var modes = mode === 'all' ? ['osu', 'taiko', 'fruits', 'mania'] : [mode], checked = 0, ranked = 0;
      return modes.reduce(function (p, m) {
        return p.then(function () {
          function chunk(off) {
            impStatus(info + '. Step 3/3: looking up official leaderboard ranks (' + m + ', ' + off + '/100)…');
            return site('/site/import_ranks', { query: query, mode: m, offset: off, count: 25 }, csrf).then(function (r) {
              checked += r.checked; ranked += r.ranked;
              return (r.next_offset === null || r.next_offset === undefined) ? null : chunk(r.next_offset);
            });
          }
          return chunk(0);
        });
      }, Promise.resolve()).then(function () { return info + ' · leaderboard ranks for ' + ranked + ' of ' + checked + ' top plays'; }, function (e) { return info + ' · rank lookup stopped: ' + e.message; });
    }).then(function (summary) {
      SG.user = name; impStatus(summary + '.', 'ok');
      setTimeout(function () { impBusy = false; $('imp').hidden = true; if (impDone) impDone(); sgAccountDone(); }, 1600);
    }).catch(function (e) {
      impStatus(e.message || String(e), 'err'); impBusy = false; $('imp-go').disabled = false; $('imp-cancel').disabled = false;
      if (impDone) impDone();
    }).then(function () { if ($('imp-cancel')) $('imp-cancel').disabled = false; });
  }
  $('imp-go').onclick = impRun;
  $('imp-cancel').onclick = impClose;
  ['imp-query', 'imp-name'].forEach(function (id) { $(id).addEventListener('keydown', function (e) { if (e.key === 'Enter') { e.preventDefault(); impRun(); } }); });

  var S = {};
  var current = '';
  var timers = [];
  function stopTimers() { timers.forEach(clearInterval); timers = []; }

  S.overview = function () {
    var up = 0, base = 0;
    function tick() { $('ov-uptime').textContent = dur(up + Math.floor((Date.now() - base) / 1000)); }
    function load() {
      api('stats').then(function (d) {
        $('ov-users').textContent = d.users;
        up = d.uptime; base = Date.now(); tick();
        $('ov-since').textContent = 'the service has been running since ' + new Date(d.started_at * 1000).toLocaleString();
        $('ov-count').textContent = '(' + d.scores_24h + ')';
        var box = $('ov-scores');
        if (!d.recent.length) { box.innerHTML = '<div class="adm-empty">No scores have been submitted in the last 24 hours.</div>'; return; }
        box.innerHTML = '<table class="adm-table"><thead><tr><th>Map</th><th>Player</th><th>Mode</th><th class="num">pp</th><th class="num">Score</th><th class="num">When</th></tr></thead><tbody>' +
          d.recent.map(function (r) {
            var name = r.title ? esc(r.artist) + ' - ' + esc(r.title) + (r.version ? ' <span class="adm-muted">[' + esc(r.version) + ']</span>' : '') : '<span class="adm-muted">unknown map</span>';
            var link = r.beatmap_id > 0 ? '<a href="/beatmaps/' + r.beatmap_id + '" target="_blank">' + name + '</a>' : name;
            return '<tr><td>' + link + '</td><td>' + esc(r.player) + '</td><td>' + (MODES[r.mode] || '') + '</td><td class="num">' + (r.pp != null ? Math.round(r.pp) : '—') +
              '</td><td class="num">' + Number(r.score).toLocaleString() + '</td><td class="num" title="' + new Date(r.time * 1000).toLocaleString() + '">' + ago(r.time) + '</td></tr>';
          }).join('') + '</tbody></table>';
      }).catch(fail);
    }
    load();
    timers.push(setInterval(tick, 1000), setInterval(load, 30000));
  };

  S.users = function () {
    function load() {
      api('users').then(function (d) {
        $('us-count').textContent = '(' + d.users.length + ')';
        if (!d.users.length) { $('us-list').innerHTML = '<div class="adm-empty">No users yet.</div>'; return; }
        $('us-list').innerHTML = '<table class="adm-table"><thead><tr><th>Name</th><th>Flag</th><th class="num">pp</th><th class="num">Scores</th><th></th></tr></thead><tbody>' +
          d.users.map(function (u) {
            return '<tr><td><b>' + esc(u.name) + '</b>' + (d.online === u.name ? ' <span class="adm-badge is-on">in-game</span>' : '') + '</td><td>' +
              (u.country ? '<span class="adm-flag">' + flagEmoji(u.country) + '</span>' + esc(u.country) : '<span class="adm-muted">none</span>') + '</td><td class="num">' + Math.round(u.pp) +
              '</td><td class="num">' + u.scores + '</td><td class="act"><button class="adm-btn" data-edit="' + esc(u.name) + '" data-cc="' + esc(u.country) + '" type="button">Edit</button> ' +
              '<button class="adm-btn adm-btn--danger" data-del="' + esc(u.name) + '" type="button">Delete</button></td></tr>';
          }).join('') + '</tbody></table>';
      }).catch(fail);
    }
    $('us-list').onclick = function (e) {
      var b = e.target.closest('button'); if (!b) return;
      if (b.dataset.edit) editUser(b.dataset.edit, b.dataset.cc, load);
      if (b.dataset.del) delUser(b.dataset.del, load);
    };
    $('us-form').onsubmit = function (e) {
      e.preventDefault();
      var name = $('us-new').value.trim(); if (!name) { toast('Enter a username.', true); return; }
      api('users/create', { name: name }).then(function (d) { $('us-new').value = ''; toast('Account "' + d.name + '" created.'); SG.user = d.name; load(); sgAccountDone(); }).catch(fail);
    };
    $('us-import').onclick = function () { impOpen(load); };
    load();
  };
  function editUser(name, cc, done) {
    var c = modal('<h2>Edit user</h2><div class="adm-hint">Changes apply without signing in to the profile.</div>' +
      '<label class="adm-field">Username<input type="text" id="eu-name" maxlength="64" value="' + esc(name) + '"></label>' +
      '<label class="adm-field">Flag (two-letter country code, empty for none)<input type="text" id="eu-cc" maxlength="2" value="' + esc(cc) + '" placeholder="GB"></label>' +
      '<div class="adm-actions"><button class="adm-btn adm-btn--primary" id="eu-save" type="button">Save</button><button class="adm-btn" id="eu-cancel" type="button">Cancel</button></div>');
    c.querySelector('#eu-cancel').onclick = closeModal;
    c.querySelector('#eu-save').onclick = function () {
      api('users/update', { name: name, new_name: $('eu-name').value, country: $('eu-cc').value }).then(function () { closeModal(); toast('User updated.'); done(); }).catch(fail);
    };
    c.querySelector('#eu-name').focus();
  }
  function delUser(name, done) {
    var c = modal('<h2>Delete ' + esc(name) + '?</h2><div class="adm-hint">This removes the profile together with all of its scores, friends and medals. It cannot be undone.</div>' +
      '<div class="adm-actions"><button class="adm-btn adm-btn--danger" id="du-ok" type="button">Delete user</button><button class="adm-btn" id="du-cancel" type="button">Cancel</button></div>');
    c.querySelector('#du-cancel').onclick = closeModal;
    c.querySelector('#du-ok').onclick = function () {
      api('users/delete', { name: name }).then(function () { closeModal(); toast('User deleted.'); done(); }).catch(function (e) { closeModal(); fail(e); });
    };
  }

  S.maps = function () {
    function load() {
      api('maps').then(function (d) {
        $('mp-count').textContent = '(' + d.maps.length + ')';
        if (!d.maps.length) { $('mp-list').innerHTML = '<div class="adm-empty">No custom ranked or loved maps yet. Use !rank or !love in-game, or the form above.</div>'; return; }
        $('mp-list').innerHTML = '<table class="adm-table"><thead><tr><th>Beatmap</th><th>Mapper</th><th>Status</th><th class="num">Diffs</th><th class="num">Updated</th><th></th></tr></thead><tbody>' +
          d.maps.map(function (m) {
            var id = m.set_id > 0 ? 'data-set="' + m.set_id + '"' : (m.beatmap_id > 0 ? 'data-bid="' + m.beatmap_id + '"' : 'data-md5="' + esc(m.md5) + '"');
            var title = esc(m.artist) + ' - ' + esc(m.title);
            var link = m.set_id > 0 ? '<a href="/beatmapsets/' + m.set_id + '" target="_blank">' + title + '</a>' : title;
            return '<tr><td>' + link + (m.set_id > 0 ? ' <span class="adm-muted">#' + m.set_id + '</span>' : '') + '</td><td>' + esc(m.creator) + '</td><td><span class="adm-badge is-' + m.status + '">' + m.status + '</span></td><td class="num">' + m.maps +
              '</td><td class="num">' + ago(m.updated) + '</td><td class="act"><button class="adm-btn" ' + id + ' data-st="ranked" type="button">Rank</button> <button class="adm-btn" ' + id + ' data-st="loved" type="button">Love</button> ' +
              '<button class="adm-btn adm-btn--danger" ' + id + ' data-st="unranked" type="button">Unrank</button></td></tr>';
          }).join('') + '</tbody></table>';
      }).catch(fail);
    }
    $('mp-list').onclick = function (e) {
      var b = e.target.closest('button[data-st]'); if (!b) return;
      setStatus({ set_id: +b.dataset.set || 0, beatmap_id: +b.dataset.bid || 0, md5: b.dataset.md5 || '', status: b.dataset.st }, load);
    };
    $('mp-form').onsubmit = function (e) {
      e.preventDefault();
      setStatus({ set_id: +$('mp-set').value || 0, beatmap_id: +$('mp-bid').value || 0, status: $('mp-status').value }, function () { $('mp-set').value = ''; $('mp-bid').value = ''; load(); });
    };
    load();
  };
  function setStatus(body, done) {
    api('maps/set', body).then(function () { toast('Status updated.'); done(); }).catch(fail);
  }

  S.keys = function () {
    var LINKS = {
      v1: ['Get a key at', 'https://old.ppy.sh/p/api/', 'Scroll to "Legacy API" and create a key (any name, e.g. localhost).'],
      v2: ['Create an OAuth application at', 'https://osu.ppy.sh/home/account/edit#oauth', 'Use any name and leave the callback URL empty; then copy the Client ID and Secret.'],
      daily: ['Get a key at', 'https://osudaily.net/api.php', 'Sign in with your osu! account to see your key.']
    };
    function card(id, title, hint, st, fields, clearName) {
      var L = LINKS[id];
      return '<div class="adm-panel" data-card="' + id + '"><div class="adm-panel__title">' + title + ' <span class="adm-badge ' + (st.set ? 'is-on' : '') + '" style="float:right">' + (st.set ? 'set' : 'not set') + '</span></div><div class="adm-panel__body">' +
        '<div class="adm-hint" style="margin-top:0">' + hint + '</div>' + fields +
        '<div class="adm-actions"><button class="adm-btn adm-btn--primary" data-save="' + id + '" type="button">Save</button>' +
        (st.set ? '<button class="adm-btn adm-btn--danger" data-remove="' + clearName + '" type="button">Remove</button>' : '') + '</div>' +
        '<div class="adm-kt" data-kt="' + id + '"></div>' +
        '<div class="adm-links">' + L[0] + ' <a href="' + L[1] + '" target="_blank" rel="noopener noreferrer">' + L[1] + '</a><div class="adm-hint">' + L[2] + '</div></div></div></div>';
    }
    function input(id, label, ph) { return '<label class="adm-field">' + label + '<input type="password" autocomplete="off" id="' + id + '" placeholder="' + esc(ph || '') + '"></label>'; }
    function load() {
      api('keys').then(function (d) {
        $('ky-grid').innerHTML =
          card('v1', 'osu! API v1', 'Legacy key used for online beatmap leaderboards and map data.', d.v1, input('k-v1', 'New key', d.v1.set ? d.v1.preview : 'paste key'), 'v1') +
          card('v2', 'osu! API v2 (OAuth)', 'OAuth application used for official score and profile imports.', { set: !!(d.client_id && d.client_secret.set) },
            '<label class="adm-field">Client ID<input type="text" id="k-cid" value="' + esc(d.client_id) + '" placeholder="12345"></label>' + input('k-csec', 'Client secret', d.client_secret.set ? d.client_secret.preview : 'paste secret'), 'v2') +
          card('daily', 'osu!daily API', 'Used for real-time global rank estimates.', d.daily, input('k-daily', 'New key', d.daily.set ? d.daily.preview : 'paste key'), 'daily');
        var l = d.ladder;
        applyTest();
        $('ky-ladder').textContent = l ? 'Data from ' + l.first_date + ' to ' + l.last_date + ' (' + l.source + ')' : 'No dataset loaded.';
      }).catch(fail);
    }
    $('ky-grid').onclick = function (e) {
      var b = e.target.closest('button'); if (!b) return;
      if (b.dataset.save) {
        var body = {};
        if (b.dataset.save === 'v1') body.v1 = $('k-v1').value;
        if (b.dataset.save === 'daily') body.daily = $('k-daily').value;
        if (b.dataset.save === 'v2') { body.client_id = $('k-cid').value; body.client_secret = $('k-csec').value; }
        api('keys', body).then(function () { toast('Saved.'); lastTest = null; load(); runTest().catch(function () {}); }).catch(fail);
      } else if (b.dataset.remove) {
        api('keys', { clear: [b.dataset.remove] }).then(function () { toast('Removed.'); lastTest = null; load(); }).catch(fail);
      }
    };
    $('ky-refresh').onclick = function () { refreshDataset(this, 'Refresh dataset', load); };
    $('ky-test').onclick = function () {
      var b = this; b.disabled = true; b.textContent = 'Testing…';
      runTest().catch(fail).then(function () { b.disabled = false; b.textContent = 'Test keys'; });
    };
    load();
  };

  var cons = { after: { packets: 0, requests: 0, internal: 0, web: 0 }, rows: { packets: 0, requests: 0, internal: 0, web: 0 } };
  S.console = function () {
    function line(chan, e) {
      if (chan === 'packets' && $('cn-hideping').checked && /OsuPing|ChoPong/.test(e.msg)) return '';
      var t = new Date(e.ts).toLocaleTimeString([], { hour12: false });
      return '<div class="l-' + esc(e.level) + '"><span class="ts">' + t + '</span>' + esc(e.msg) + '</div>';
    }
    function poll() {
      Promise.all(['packets', 'requests', 'internal', 'web'].map(function (chan) {
        return api('console?chan=' + chan + '&after=' + cons.after[chan]).then(function (d) {
          var list = d[chan] || []; if (!list.length) return;
          var box = $('term-' + chan), stick = box.scrollTop + box.clientHeight >= box.scrollHeight - 30, html = '';
          list.forEach(function (e) { cons.after[chan] = Math.max(cons.after[chan], e.id); html += line(chan, e); });
          if (html) { box.insertAdjacentHTML('beforeend', html); while (box.children.length > 600) box.removeChild(box.firstChild); if (stick) box.scrollTop = box.scrollHeight; }
        });
      })).catch(function () {});
    }
    document.querySelectorAll('[data-clear]').forEach(function (b) {
      b.onclick = function () { var c = b.dataset.clear; $('term-' + c).innerHTML = ''; api('console/clear?chan=' + c, {}).catch(fail); };
    });
    poll(); timers.push(setInterval(poll, 1500));
  };

  var PANELS = [
    { id: 'lb', title: 'Leaderboards & chat', fields: [
      { k: 'amount_of_scores_on_lb', label: 'Scores shown on in-game leaderboards (1–100)', type: 'number', min: 1, max: 100, hint: '50 and 100 are the usual choices.' },
      { k: 'pp_leaderboard', label: 'Show pp instead of score on leaderboards', type: 'bool' },
      { k: 'show_pp_for_personal_best', label: 'Show pp in the personal best panel', type: 'bool' },
      { k: 'enable_recent_channel', label: 'Post new scores in the #recent chat channel', type: 'bool' },
      { k: 'ping_user_when_recent_score', label: 'Ping me when a #recent score is posted', type: 'bool' },
      { k: 'command_prefix', label: 'Chat command prefix', type: 'text', max: 3, hint: 'Default is !' },
      { k: 'disable_funorange_maps', label: 'Hide FunOrange maps', type: 'bool' },
      { k: 'auto_update', label: 'Update automatically', type: 'bool' } ] },
    { id: 'folders', title: 'Folders', note: 'Folder changes apply after restarting the server.', fields: [
      { k: 'osu_path', label: 'osu! folder', type: 'text', hint: 'Songs, Replays and Screenshots are looked up here by default.' },
      { k: 'songs', label: 'Songs folder (optional override)', type: 'text' },
      { k: 'replay', label: 'Replays folder (optional override)', type: 'text' },
      { k: 'screenshots', label: 'Screenshots folder (optional override)', type: 'text' } ] },
    { id: 'profile', title: 'Profile & official account', fields: [
      { k: 'country', label: 'Default country code (e.g. US, DE, JP)', type: 'text', max: 2 },
      { k: 'osu_username', label: 'osu! username (only for !friend sync)', type: 'text' },
      { k: 'osu_password', label: 'osu! password (stored only as an MD5 hash)', type: 'secret', setKey: 'osu_password_set' } ] },
    { id: 'discord', title: 'Discord', note: 'The bot settings apply after restarting the server.', fields: [
      { k: 'discord_webhook_url', label: 'Score webhook URL', type: 'text' },
      { k: 'discord_webhook_min_pp', label: 'Minimum pp to post', type: 'text' },
      { k: 'discord_bot_token', label: 'Bot token', type: 'secret', maskKey: true },
      { k: 'discord_application_id', label: 'Bot application ID', type: 'text' },
      { k: 'discord_guild_id', label: 'Server (guild) ID, optional', type: 'text' } ] },
    { id: 'server', title: 'Server', note: 'Changing the address or port needs a restart of the server (and of osu!).', fields: [
      { k: 'host', label: 'Listen address', type: 'text' },
      { k: 'port', label: 'Port', type: 'number', min: 1, max: 65535 } ] }
  ];
  var settingsCache = {};
  function panelHtml(p, d) {
    var f = p.fields.map(function (f) {
      var id = 'st-' + f.k, v = d[f.k];
      if (f.type === 'bool') return '<label class="adm-check2"><input type="checkbox" id="' + id + '"' + (v ? ' checked' : '') + '> ' + f.label + '</label>';
      if (f.type === 'secret') {
        var set = f.maskKey ? (v && v.set) : d[f.setKey];
        return '<label class="adm-field">' + f.label + '<input type="password" autocomplete="off" id="' + id + '" placeholder="' + (set ? (f.maskKey ? v.preview : 'set — type to replace') : 'not set') + '"></label>';
      }
      return '<label class="adm-field">' + f.label + '<input type="' + (f.type === 'number' ? 'number' : 'text') + '" id="' + id + '"' + (f.min != null ? ' min="' + f.min + '"' : '') + (f.max != null && f.type === 'number' ? ' max="' + f.max + '"' : '') +
        (f.max != null && f.type !== 'number' ? ' maxlength="' + f.max + '"' : '') + ' value="' + esc(v == null ? '' : v) + '">' + (f.hint ? '<span class="adm-hint" style="margin:0">' + f.hint + '</span>' : '') + '</label>';
    }).join('');
    return '<div class="adm-panel" data-panel="' + p.id + '"><div class="adm-panel__title">' + p.title + '</div><div class="adm-panel__body"><div class="adm-set">' + f + '</div>' +
      (p.note ? '<div class="adm-set__note">' + p.note + '</div>' : '') + '<div class="adm-actions"><button class="adm-btn adm-btn--primary" data-psave="' + p.id + '" type="button">Save</button></div></div></div>';
  }
  S.other = function () {
    api('other').then(function (d) {
      $('ot-theme').innerHTML = d.themes.map(function (t) { return '<option value="' + esc(t.id) + '"' + (t.id === d.theme ? ' selected' : '') + '>' + esc(t.name) + '</option>'; }).join('');
    }).catch(fail);
    $('ot-save').onclick = function () {
      api('other/theme', { theme: $('ot-theme').value }).then(function () { toast('Default theme saved.'); }).catch(fail);
    };
    api('settings').then(function (d) {
      settingsCache = d;
      var html = PANELS.map(function (p) { return panelHtml(p, d); }).join('');
      if (d.windows) html += '<div class="adm-panel"><div class="adm-panel__title">Certificate & hosts file (Windows)</div><div class="adm-panel__body"><div class="adm-row"><div class="adm-hint" style="margin:0">Trusts the local HTTPS certificate and adds the osu.* entries to the hosts file, so <code>-devserver localhost</code> works. Needs administrator rights.</div><button class="adm-btn" id="st-cert" type="button">Install / repair</button></div></div></div>';
      $('ot-settings').innerHTML = html;
    }).catch(fail);
    $('ot-settings').onclick = function (e) {
      var b = e.target.closest('button'); if (!b) return;
      if (b.id === 'st-cert') { b.disabled = true; api('system/trust-cert', {}).then(function (d) { toast(d.message); }).catch(fail).then(function () { b.disabled = false; }); return; }
      var pid = b.dataset.psave; if (!pid) return;
      var p = PANELS.filter(function (x) { return x.id === pid; })[0], body = {};
      p.fields.forEach(function (f) {
        var el = $('st-' + f.k); if (!el) return;
        if (f.type === 'bool') body[f.k] = el.checked;
        else if (f.type === 'secret') { if (el.value) body[f.k] = el.value; }
        else if (f.type === 'number') { if (el.value !== '') body[f.k] = f.k === 'amount_of_scores_on_lb' ? +el.value : el.value; }
        else body[f.k] = el.value;
      });
      b.disabled = true;
      api('settings', body).then(function (d) { toast(d.restart_needed ? 'Saved. Restart the server for the changed folder/Discord/address settings to take effect.' : 'Saved.'); }).catch(fail).then(function () { b.disabled = false; });
    };
  };

  function show(name) {
    if (!S[name]) name = 'overview';
    if (name === current) return;
    stopTimers(); current = name;
    document.querySelectorAll('.adm-section').forEach(function (s) { s.hidden = s.dataset.section !== name; });
    document.querySelectorAll('.adm-side__item').forEach(function (a) { a.classList.toggle('is-active', a.dataset.section === name); });
    S[name]();
  }
  window.addEventListener('hashchange', function () { show(location.hash.slice(1)); });

  fetch('/site/session', { credentials: 'same-origin' }).then(function (r) { return r.json(); }).then(function () {
    var p = $('adm-status'); p.textContent = 'connected'; p.className = 'adm-pill is-ok';
  }).catch(function () { var p = $('adm-status'); p.textContent = 'offline'; p.className = 'adm-pill is-err'; });
  show(location.hash.slice(1));
  sgLoad();
})();
