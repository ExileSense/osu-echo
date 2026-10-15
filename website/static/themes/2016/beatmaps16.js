(function () {
  'use strict';
  var BASE = '/site/static/themes/2016/';
  var $ = function (id) { return document.getElementById(id); };
  var esc = function (s) { return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) { return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]; }); };
  var num = function (n) { try { return Number(n || 0).toLocaleString('en-US'); } catch (e) { return String(n || 0); } };
  var fix = function (n, d) { return (Number(n) || 0).toFixed(d); };
  var json = function (u) { return fetch(u, { credentials: 'same-origin' }).then(function (r) { return r.json().catch(function () { return {}; }).then(function (j) { if (!r.ok) throw new Error((j && j.error) || String(r.status)); return j; }); }); };
  var page = $('t16-page');
  var session = { user: null };
  var MODE_KEY = ['osu', 'taiko', 'fruits', 'mania'];

  function css(file) { var l = document.createElement('link'); l.rel = 'stylesheet'; l.href = BASE + 'css/' + file + '?v=1'; document.head.appendChild(l); }
  function ago(ts) {
    var t = typeof ts === 'string' && isNaN(Number(ts)) ? Date.parse(ts) / 1000 : Number(ts) || 0; if (t > 1e12) t /= 1000; if (!t) return '';
    var d = Math.max(0, Math.round(Date.now() / 1000 - t)), u = [['year', 31536000], ['month', 2592000], ['day', 86400], ['hour', 3600], ['minute', 60]];
    for (var i = 0; i < u.length; i++) if (d >= u[i][1]) { var n = Math.floor(d / u[i][1]); return n + ' ' + u[i][0] + (n > 1 ? 's' : '') + ' ago'; }
    return 'just now';
  }
  var MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  function dshort(iso) { var t = Date.parse(iso); if (isNaN(t)) return ''; var d = new Date(t); return MONTHS[d.getUTCMonth()] + ' ' + d.getUTCDate() + ', ' + d.getUTCFullYear(); }
  function flag(c) { c = String(c || '').toLowerCase(); return /^[a-z]{2}$/.test(c) ? '<span class="t16-flag f-' + c + '" title="' + c.toUpperCase() + '"></span> ' : ''; }
  function grade(g, small) { g = String(g || 'D').toUpperCase(); if (g === 'F') g = 'D'; if (['X', 'XH', 'S', 'SH', 'A', 'B', 'C', 'D'].indexOf(g) < 0) g = 'D'; return BASE + 'images/grades/' + g + (small ? '_small' : '') + '.png'; }
  function modsShort(m) { m = String(m || '').replace(/^\+/, '').replace(/[^A-Za-z0-9]/g, ''); return !m || m === 'NM' || m === 'None' ? 'None' : m; }
  function diffAlias(r) { r = Number(r) || 0; return r < 2 ? 'easy' : r < 2.7 ? 'normal' : r < 4 ? 'hard' : r < 5.3 ? 'insane' : 'expert'; }
  function diffIcon(b) { return BASE + 'images/beatmap/difficulties/' + diffAlias(b.stars) + '-' + (b.mode || 0) + '.png'; }
  function ratingAvg(r) { var t = 0, s = 0; (r || []).forEach(function (v, i) { t += v; s += v * i; }); return t ? s / t : 0; }

  function chrome(s) {
    var box = $('t16-login'), n = (s.profiles || []).length;
    if (s.user) box.innerHTML = '<span class="nav-login">Welcome, <b><a href="/users/' + encodeURIComponent(s.user) + '">' + esc(s.user) + '</a></b></span> <img class="small-avatar" src="/site/avatar?name=' + encodeURIComponent(s.user) + '" alt=""> <a href="/users/' + encodeURIComponent(s.user) + '"><i class="icon-user" style="color:#808080"></i> Profile</a> <a href="/profile#login"><i class="icon-exchange" style="color:#808080"></i> Switch profile</a>';
    else box.innerHTML = '<span class="nav-login">Welcome, guest!</span> <a href="/profile#login"><i class="icon-caret-down"></i> Login</a>';
    $('t16-stats').innerHTML = '<b>' + num(n) + '</b> local profile' + (n === 1 ? '' : 's') + ', <b>' + (s.active ? 1 : 0) + '</b> online now.<br>Local osu! server';
  }
  $('t16-user-search').addEventListener('submit', function (e) { e.preventDefault(); var v = $('user-search').value.trim(); if (v) location.href = '/users/' + encodeURIComponent(v) + '?mode=osu'; });
  $('t16-map-search').addEventListener('submit', function (e) { e.preventDefault(); var v = $('beatmap-search').value.trim(); if (v) location.href = '/beatmapsets?q=' + encodeURIComponent(v); });

  var audio = null, audioId = null;
  function preview(id, btn) {
    if (audio) { audio.pause(); audio = null; }
    if (audioId === id) { audioId = null; return; }
    audioId = id; audio = new Audio('https://b.ppy.sh/preview/' + id + '.mp3'); audio.volume = 0.4;
    audio.onended = audio.onerror = function () { audio = null; audioId = null; };
    audio.play().catch(function () { audio = null; audioId = null; });
  }

  var L = { q: '', m: '', s: 'leaderboard', sort: 'ranked_desc', page: 0, items: [], more: false, busy: false, error: '' };
  var MODES = [['', 'Any'], ['0', 'osu!'], ['1', 'osu!taiko'], ['2', 'osu!catch'], ['3', 'osu!mania']];
  var CATS = [['leaderboard', 'Has Leaderboard'], ['ranked', 'Ranked'], ['qualified', 'Qualified'], ['loved', 'Loved'], ['pending', 'Pending'], ['graveyard', 'Graveyard'], ['any', 'All']];
  var SORTS = [['title', 'Title', 'title_asc', 'title_desc'], ['artist', 'Artist', 'artist_asc', 'artist_desc'], ['ranked', 'Ranked', 'ranked_desc', 'ranked_asc'], ['updated', 'Updated', 'updated_desc', 'updated_asc'], ['favourites', 'Favourites', 'favourites_desc', 'favourites_asc'], ['plays', 'Plays', 'plays_desc', 'plays_asc']];
  function readQuery() { var p = new URLSearchParams(location.search); L.q = p.get('q') || ''; L.m = p.get('m') || ''; L.s = p.get('s') || 'leaderboard'; L.sort = p.get('sort') || 'ranked_desc'; L.page = Math.max(0, parseInt(p.get('page') || '0', 10) || 0); }
  function pushQuery() {
    var p = new URLSearchParams(); if (L.q) p.set('q', L.q); if (L.m !== '') p.set('m', L.m); if (L.s !== 'leaderboard') p.set('s', L.s); if (L.sort !== 'ranked_desc') p.set('sort', L.sort); if (L.page) p.set('page', L.page);
    var qs = p.toString(); try { history.replaceState(null, '', '/beatmapsets' + (qs ? '?' + qs : '')); } catch (e) { }
  }
  var STATUS_LABEL = { ranked: 'Ranked', approved: 'Approved', loved: 'Loved', qualified: 'Qualified', pending: 'Pending', graveyard: 'Graveyard', wip: 'WIP' };
  function item(set) {
    var id = set.id, maps = (set.beatmaps || []).slice().sort(function (a, b) { return (a.mode || 0) - (b.mode || 0) || a.stars - b.stars; });
    var icons = maps.slice(0, 8).map(function (b) { return '<a href="/beatmapsets/' + id + '#' + MODE_KEY[b.mode || 0] + '/' + b.id + '"><img class="beatmap-icon" src="' + diffIcon(b) + '" title="' + esc(b.version) + '" alt="' + esc(b.version) + '"></a>'; }).join('') + (maps.length > 8 ? '...' : '');
    var tags = (set.language && set.language !== 'Unspecified' ? '<a>' + esc(set.language) + '</a>' : '') + (set.genre && set.genre !== 'Unspecified' ? '<a>' + esc(set.genre) + '</a>' : '');
    var date = set.ranked_date || set.submitted_date || set.last_updated;
    return '<div class="beatmapset" id="beatmapset-' + id + '"><div class="beatmap-image" data-prev="' + id + '" style="background-image:url(https://assets.ppy.sh/beatmaps/' + id + '/covers/list.jpg)"><i class="icon-play"></i></div>' +
      '<div class="beatmap-side-options" style="overflow:hidden;width:24px;display:none"><a class="beatmap-download-link" href="/d/' + id + '" title="Download"><i class="icon-download-alt"></i></a></div>' +
      '<div class="beatmap-info">' + (set.video ? '<i class="icon-film"></i>' : '') + '<a class="beatmap-link" href="/beatmapsets/' + id + '"><span style="color:#555555">' + esc(set.artist) + '</span> - <span class="beatmap-title">' + esc(set.title) + '</span></a></div>' +
      '<div class="beatmap-creator"><span>mapped by </span><a href="/beatmapsets?q=' + encodeURIComponent(set.creator || '') + '">' + esc(set.creator) + '</a></div>' +
      '<div class="beatmap-stats"><div class="beatmap-tags">' + tags + '</div><div class="beatmap-rating"><div class="beatmap-rating-bar" style="width:' + Math.min(100, ratingAvg(set.ratings) * 10) + '%"></div></div>' +
      '<div class="beatmap-details"><span class="hidden-elements">' + esc(dshort(date)) + '</span> <i class="icon-heart"></i>' + num(set.favourites) + ' <i class="icon-play"></i>' + num(set.plays) + '</div></div>' +
      '<div class="hidden-elements"><div class="beatmap-source">' + (set.source ? 'from <span>' + esc(set.source) + '</span>' : '') + '</div>' + icons + '</div></div>';
  }
  function dl(title, key, opts) {
    return '<dl><dt>' + title + '</dt><dd>' + opts.map(function (o) { return '<a data-f="' + key + '" data-v="' + esc(o[0]) + '"' + (String(L[key]) === String(o[0]) ? ' class="selected"' : '') + '>' + esc(o[1]) + '</a>'; }).join('') + '</dd></dl>';
  }
  function drawList() {
    var from = L.page * 50 + 1, to = L.page * 50 + L.items.length;
    var pager = '<div style="text-align:center;font-size:80%;margin:6px 0">' + (L.items.length ? 'Displaying ' + from + ' to ' + to + ' &nbsp;' : '') + (L.page > 0 ? '<a data-pg="-1">&laquo; Previous</a> ' : '') + '<b>Page ' + (L.page + 1) + '</b>' + (L.more ? ' <a data-pg="1">Next &raquo;</a>' : '') + '</div>';
    var order = '<table class="beatmap-order-select"><thead><tr>' + SORTS.map(function (s) {
      var on = L.sort === s[2] || L.sort === s[3], desc = L.sort === s[2];
      var arrow = on ? '<img src="' + BASE + 'images/' + ((s[0] === 'title' || s[0] === 'artist') ? (desc ? 'down' : 'up') : (desc ? 'down' : 'up')) + '.gif" alt="">' : '';
      return '<th><a data-sort="' + s[0] + '">' + s[1] + '</a> ' + arrow + '</th>';
    }).join('') + '</tr></thead></table>';
    var body = L.busy && !L.items.length ? '<div id="status-text" style="padding:20px">Loading...</div>' : (!L.items.length ? '<div id="status-text" style="padding:20px">' + (L.error ? esc(L.error) : 'No beatmaps found.') + '</div>' : '<div id="beatmap-list">' + L.items.map(item).join('') + '</div><div style="clear:both"></div>');
    $('t16-list').innerHTML = pager + order + body + pager;
  }
  function load() {
    L.busy = true; L.error = ''; drawList();
    var qs = 'q=' + encodeURIComponent(L.q) + '&s=' + encodeURIComponent(L.s) + '&sort=' + encodeURIComponent(L.sort) + '&page=' + L.page + (L.m !== '' ? '&m=' + L.m : '');
    return json('/site/beatmaps?' + qs).then(function (d) { L.items = d.sets || []; L.more = !!d.has_more; }).catch(function (e) { L.items = []; L.more = false; L.error = e.message || 'Could not load beatmaps.'; }).then(function () { L.busy = false; drawList(); });
  }
  function listing() {
    css('search.css'); readQuery(); document.title = 'Beatmap Listing';
    page.innerHTML = '<div class="beatmap-search-container"><h1>Beatmap Listing</h1><form id="beatmap-search-form"><input id="search-input" name="query" type="search" placeholder="Search for a beatmap..." value="' + esc(L.q) + '"></form>' +
      '<div class="beatmap-options" id="t16-opts"></div><div id="t16-list"></div></div>';
    function opts() { $('t16-opts').innerHTML = dl('Mode:', 'm', MODES) + dl('Category:', 's', CATS); }
    opts();
    $('beatmap-search-form').addEventListener('submit', function (e) { e.preventDefault(); L.q = $('search-input').value.trim(); L.page = 0; pushQuery(); load(); });
    page.addEventListener('click', function (e) {
      var f = e.target.closest('[data-f]'); if (f) { L[f.getAttribute('data-f')] = f.getAttribute('data-v'); L.page = 0; opts(); pushQuery(); load(); return; }
      var s = e.target.closest('[data-sort]'); if (s) { var k = s.getAttribute('data-sort'), d = SORTS.filter(function (x) { return x[0] === k; })[0]; L.sort = L.sort === d[2] ? d[3] : d[2]; L.page = 0; pushQuery(); load(); return; }
      var p = e.target.closest('[data-pg]'); if (p) { L.page = Math.max(0, L.page + Number(p.getAttribute('data-pg'))); pushQuery(); load(); window.scrollTo(0, 0); return; }
      var im = e.target.closest('[data-prev]'); if (im) preview(im.getAttribute('data-prev'));
    });
    page.addEventListener('mouseover', function (e) { var b = e.target.closest('.beatmapset'); if (b && !b.classList.contains('t16-hover')) { b.classList.add('t16-hover'); b.querySelectorAll('.hidden-elements').forEach(function (x) { x.style.display = 'block'; }); var so = b.querySelector('.beatmap-side-options'); if (so) so.style.display = 'block'; } });
    page.addEventListener('mouseout', function (e) { var b = e.target.closest('.beatmapset'); if (b && !b.contains(e.relatedTarget)) { b.classList.remove('t16-hover'); b.querySelectorAll('.hidden-elements').forEach(function (x) { x.style.display = ''; }); var so = b.querySelector('.beatmap-side-options'); if (so) so.style.display = 'none'; } });
    load();
  }

  var S = null, sel = null, BOARD = { mods: [] };
  var MODS = [['Difficulty Reduction', [['EZ', 'Easy'], ['NF', 'No Fail'], ['HT', 'Half Time']]], ['Difficulty Increase', [['HR', 'Hard Rock'], ['SD', 'Sudden Death'], ['PF', 'Perfect'], ['DT', 'Double Time'], ['NC', 'Nightcore'], ['HD', 'Hidden'], ['FL', 'Flashlight']]], ['Special', [['SO', 'Spun Out'], ['TD', 'Touch Device']]]];
  function stars(v, max) { return '<div class="starfield" style="width:140px"><div class="starfield-selected" style="width:' + (14 * Math.max(0, Math.min(10, Number(v) || 0))) + 'px"></div></div>'; }
  function leaderCard(s, rank, mode) {
    var hits = mode === 3 ? '<tr class="row-light"><td><b>MAX / 300 / 200</b></td><td>' + s.geki + ' / ' + s.n300 + ' / ' + s.katu + '</td></tr><tr class="row-dark"><td><b>100 / 50 / Misses</b></td><td>' + s.n100 + ' / ' + s.n50 + ' / ' + s.miss + '</td></tr>' :
      '<tr class="row-light"><td><b>300 / 100 / 50</b></td><td>' + s.n300 + ' / ' + s.n100 + ' / ' + s.n50 + '</td></tr><tr class="row-dark"><td><b>Misses</b></td><td>' + s.miss + '</td></tr><tr class="row-light"><td><b>Geki (Elite Beat!)</b></td><td>' + s.geki + '</td></tr><tr class="row-dark"><td><b>' + (mode === 2 ? 'Droplet misses' : 'Katu (Beat!)') + '</b></td><td>' + s.katu + '</td></tr>';
    var acc = s.acc > 1.0001 ? s.acc : s.acc * 100;
    return '<table class="leader"><tbody><tr><td class="row-title" colspan="3">' + (rank === 0 ? flag(s.country) + '<a href="/users/' + encodeURIComponent(s.player) + '">' + esc(s.player) + '</a> is in the lead! (' + esc(ago(s.time)) + ')' : 'Personal Best (#' + s.pos + ' achieved ' + esc(ago(s.time)) + ')') + '</td></tr>' +
      '<tr class="row-light"><td><b>Score</b></td><td>' + num(s.score) + ' (' + fix(acc, 2) + '%)</td><td class="row-grade" rowspan="13"><img src="' + grade(s.grade) + '" alt="Grade ' + esc(s.grade) + '"></td></tr>' +
      '<tr class="row-dark"><td><b>Max Combo</b></td><td>' + num(s.combo) + '</td></tr>' + hits + '<tr class="row-light"><td><b>Mods</b></td><td>' + esc(modsShort(s.mods)) + '</td></tr></tbody></table>';
  }
  function scoresTable(sc, mode, bm) {
    var th = mode === 3 ? '<th><b>MAX</b></th><th><b>300</b></th><th><b>200</b></th><th><b>100</b></th><th><b>50</b></th>' : '<th><b>300 / 100 / 50</b></th><th><b>Geki</b></th><th><b>' + (mode === 2 ? 'Droplet Miss' : 'Katu') + '</b></th>';
    var rows = sc.map(function (s, i) {
      var cls = s.self ? 'row-self' : s.friend ? 'row-friend' : (i % 2 ? 'row-dark' : 'row-light'), acc = s.acc > 1.0001 ? s.acc : s.acc * 100;
      var hits = mode === 3 ? '<td>' + s.geki + '</td><td>' + s.n300 + '</td><td>' + s.katu + '</td><td>' + s.n100 + '</td><td>' + s.n50 + '</td>' : '<td>' + s.n300 + ' / ' + s.n100 + ' / ' + s.n50 + '</td><td>' + s.geki + '</td><td>' + s.katu + '</td>';
      return '<tr class="' + cls + '" data-score="' + s.id + '"><td><a href="#' + s.pos + '">#' + s.pos + '</a></td><td><img src="' + grade(s.grade, true) + '" alt=""></td><td>' + (i === 0 ? '<b>' + num(s.score) + '</b>' : num(s.score)) + '</td><td>' + (acc >= 99.995 ? '<b>' + fix(acc, 2) + '%</b>' : fix(acc, 2) + '%') + '</td>' +
        '<td>' + flag(s.country) + '<a href="/users/' + encodeURIComponent(s.player) + '">' + esc(s.player) + '</a></td><td>' + (bm.max_combo && s.combo === bm.max_combo ? '<b>' + num(s.combo) + '</b>' : num(s.combo)) + '</td><td>' + (s.pp > 0 ? num(Math.round(s.pp)) : '-') + '</td>' + hits + '<td>' + s.miss + '</td><td>' + esc(modsShort(s.mods)) + '</td></tr>';
    }).join('');
    return '<table class="scores"><thead class="scores-header"><tr><th></th><th><b>Rank</b></th><th><b>Score</b></th><th><b>Accuracy</b></th><th><b>Player</b></th><th><b>Max Combo</b></th><th><b>PP</b></th>' + th + '<th><b>Misses</b></th><th><b>Mods</b></th></tr></thead><tbody>' + rows + '</tbody></table>';
  }
  function modSelector() {
    return '<div class="spoiler beatmap-mod-selector"><button type="button" class="spoiler-head beatmap-mod-selector-toggle" id="t16-modtoggle">Select Mods</button><div class="spoiler-body" id="t16-modbody" style="display:' + (BOARD.mods.length ? 'block' : 'none') + '"><div class="beatmap-mod-groups">' +
      MODS.map(function (g) { return '<div class="beatmap-mod-group"><b class="beatmap-mod-group-name">' + g[0] + '</b><span class="beatmap-mod-options">' + g[1].map(function (m) { return '<label class="beatmap-mod-option"><input type="checkbox" data-mod="' + m[0] + '"' + (BOARD.mods.indexOf(m[0]) >= 0 ? ' checked' : '') + '> ' + m[1] + '</label>'; }).join('') + '</span></div>'; }).join('') +
      '</div><div class="beatmap-mod-actions"><input type="button" id="t16-modapply" value="Apply"></div></div></div>';
  }
  function board(mode) {
    var b = sel, el = $('t16-board'); if (!el) return;
    el.innerHTML = '<div class="score-header"><b class="no-records">Loading...</b></div>';
    var mp = BOARD.mods.length ? BOARD.mods.join(',') : '';
    json('/site/beatmap_scores?id=' + b.id + '&md5=' + encodeURIComponent(b.checksum || '') + '&m=' + mode + '&type=global' + (mp ? '&mods=' + encodeURIComponent(mp) : '')).then(function (d) {
      if (sel !== b) return;
      var sc = d.scores || [], h;
      if (d.requires_login) h = '<div class="score-header"><b class="no-records">Please log in to see the leaderboard.</b></div>';
      else if (!sc.length) h = '<div class="score-header"><b class="no-records">No records have been set for this game mode' + (mp ? ' and mod combination' : '') + ' yet!</b></div>' + (mp ? '<h2>Top 100 Scoreboard (' + mp + ')</h2>' + modSelector() : '');
      else h = '<div class="score-header">' + leaderCard(sc[0], 0, mode) + (d.me && d.me.pos > 1 ? leaderCard(d.me, d.me.pos, mode) : '') + '</div><h2>Top ' + sc.length + ' Scoreboard' + (mp ? ' (' + mp + ')' : '') + '</h2>' + modSelector() + scoresTable(sc, mode, b);
      el.innerHTML = h;
    }).catch(function () { el.innerHTML = '<div class="score-header"><b class="no-records">Could not load scores.</b></div>'; });
  }
  function modeTabs(mode) {
    var names = ['osu! Standard', 'Taiko', 'Catch The Beat', 'osu!mania'];
    return '<div class="tab-list"><ul>' + names.map(function (n, i) { return '<li class="tab-item"><a href="#" data-mode="' + i + '" class="tab-link ' + (i === mode ? 'active' : '') + '">' + n + '</a></li>'; }).join('') + '</ul></div>';
  }
  function showSet() {
    var set = S, b = sel, mode = b.mode || 0, maps = (set.beatmaps || []).filter(function (x) { return (x.mode || 0) === mode; }).sort(function (a, c) { return a.stars - c.stars; });
    var len = b.length || 0, mm = Math.floor(len / 60), ss = len % 60, ranked = ['ranked', 'approved', 'loved', 'qualified'].indexOf(set.status) >= 0;
    var tabs = '<div class="tab-list"><ul>' + maps.map(function (m) { return '<li class="tab-item difficulty-icon"><a href="/beatmapsets/' + set.id + '#' + MODE_KEY[m.mode || 0] + '/' + m.id + '" data-map="' + m.id + '" class="tab-link ' + (m.id === b.id ? 'active' : '') + '"><img src="' + diffIcon(m) + '" alt="" width="16" height="16"> <span>' + esc(m.version) + '</span></a></li>'; }).join('') + '</ul></div>';
    var h = '<h1 class="header-text"><span><a href="/beatmapsets">Beatmap Listing</a> &raquo; ' + esc(set.artist) + ' - ' + esc(set.title) + '</span><p>View on <a href="https://osu.ppy.sh/beatmapsets/' + set.id + '" target="_blank" rel="noreferrer">osu.ppy.sh</a></p></h1><div class="beatmap">' + tabs +
      '<div class="beatmap-stats"><table>' +
      '<tr><td>Artist:</td><td class="beatmap-stats-info"><a href="/beatmapsets?q=' + encodeURIComponent(set.artist || '') + '">' + esc(set.artist) + '</a></td><td>Circle Size:</td><td class="beatmap-stats-info">' + stars(b.cs) + '</td><td>Approach Rate</td><td class="beatmap-stats-info">' + stars(b.ar) + '</td></tr>' +
      '<tr><td>Title:</td><td class="beatmap-stats-info"><a href="/beatmapsets?q=' + encodeURIComponent(set.title || '') + '">' + esc(set.title) + '</a></td><td>HP Drain:</td><td class="beatmap-stats-info">' + stars(b.hp) + '</td><td><b>Star Difficulty:</b></td><td class="beatmap-stats-info">' + stars(b.stars) + ' (' + fix(b.stars, 2) + ')</td></tr>' +
      '<tr><td>Creator:</td><td class="beatmap-stats-info"><a href="/beatmapsets?q=' + encodeURIComponent(set.creator || '') + '">' + esc(set.creator) + '</a></td><td>Accuracy:</td><td class="beatmap-stats-info">' + stars(b.od) + '</td><td>Length:</td><td class="beatmap-stats-info">' + mm + ':' + (ss < 10 ? '0' : '') + ss + '</td></tr>' +
      '<tr><td>Source:</td><td class="beatmap-stats-info">' + (set.source ? '<a href="/beatmapsets?q=' + encodeURIComponent(set.source) + '">' + esc(set.source) + '</a>' : '') + '</td><td>Genre:</td><td class="beatmap-stats-info">' + esc(set.genre || '') + (set.language && set.language !== 'Unspecified' ? ' (' + esc(set.language) + ')' : '') + '</td><td>BPM:</td><td class="beatmap-stats-info">' + Math.round(b.bpm || set.bpm || 0) + '</td></tr>' +
      '<tr><td>Submitted:<br>' + (ranked ? (STATUS_LABEL[set.status] || set.status) + ':' : 'Last Updated:') + '</td><td class="beatmap-stats-info"><span>' + esc(dshort(set.submitted_date)) + '</span><br><span>' + esc(dshort(ranked ? (set.ranked_date || set.last_updated) : set.last_updated)) + '</span></td><td>Average Rating:</td><td class="beatmap-stats-info">' + (ratingAvg(set.ratings) ? fix(ratingAvg(set.ratings), 4) : 'Nobody has rated this map yet... ):') + '</td><td><b>Favourited ' + num(set.favourites) + ' time' + (set.favourites === 1 ? '' : 's') + '</b> in total</td><td class="beatmap-stats-info">Played ' + num(set.plays) + ' times</td></tr>' +
      '<tr><td>Options:</td><td colspan="5"><p><a href="https://osu.ppy.sh/beatmapsets/' + set.id + '/discussion" target="_blank" rel="noreferrer">Modding / Discussion</a></p><p><a id="t16-copyid" setid="' + set.id + '">Copy Beatmapset ID</a></p><p><a href="/d/' + set.id + '">Download (local)</a></p></td></tr></table></div>' +
      '<div class="padding"><div class="beatmap-download"><a href="/d/' + set.id + '"><img src="' + BASE + 'images/beatmap/osu-download-beatmap.png" alt="Download"></a>' + (set.video ? '<a href="/d/' + set.id + 'n"><img src="' + BASE + 'images/beatmap/osu-download-beatmap-novid.png" alt="Download without Video"></a>' : '') + '</div>' +
      '<div class="beatmap-description"><div title="Play Song"><a data-prev="' + set.id + '" class="beatmap-preview-link" href="javascript:void(0);"><img src="https://b.ppy.sh/thumb/' + set.id + 'l.jpg" alt="" width="160" height="120"></a></div>' + descHtml(set.description) + '</div></div>' +
      (ranked ? modeTabs(mode) + '<div id="t16-board"></div>' : '') + '</div>';
    page.innerHTML = h; document.title = set.artist + ' - ' + set.title + ' - Beatmap Info';
    if (ranked) board(mode);
  }
  function descHtml(d) { if (!d) return ''; return window.Bbcode ? '<div class="bbcode">' + window.Bbcode.render(d) + '</div>' : '<div class="bbcode">' + esc(d) + '</div>'; }
  function pickInitial(set) {
    var h = (location.hash || '').replace(/^#/, '').split('/'), maps = set.beatmaps || [];
    if (h[1]) { var x = maps.filter(function (m) { return String(m.id) === h[1]; })[0]; if (x) return x; }
    if (h[0]) { var mi = MODE_KEY.indexOf(h[0]); if (mi >= 0) { var l = maps.filter(function (m) { return (m.mode || 0) === mi; }); if (l.length) return l[l.length - 1]; } }
    return maps[0] || null;
  }
  function sync() { try { history.replaceState(null, '', '/beatmapsets/' + S.id + '#' + MODE_KEY[sel.mode || 0] + '/' + sel.id); } catch (e) { } }
  function setPage(id, bid) {
    css('beatmap.css');
    json('/site/beatmapset?' + (id ? 'id=' + id : 'bid=' + bid)).then(function (d) {
      S = d.set; if (!S) throw new Error('Beatmapset not found.');
      sel = bid ? ((S.beatmaps || []).filter(function (x) { return String(x.id) === String(bid); })[0] || null) : null; if (!sel) sel = pickInitial(S); if (!sel) throw new Error('This beatmapset has no difficulties.');
      sync(); showSet();
      page.addEventListener('click', function (e) {
        var im = e.target.closest('[data-prev]'); if (im) { preview(im.getAttribute('data-prev')); return; }
        var m = e.target.closest('[data-mode]'); if (m) { e.preventDefault(); var mm = Number(m.getAttribute('data-mode')), l = S.beatmaps.filter(function (x) { return (x.mode || 0) === mm; }); if (l.length) { sel = l[l.length - 1]; BOARD.mods = []; sync(); showSet(); } return; }
        var p = e.target.closest('[data-map]'); if (p) { e.preventDefault(); sel = S.beatmaps.filter(function (x) { return String(x.id) === p.getAttribute('data-map'); })[0] || sel; sync(); showSet(); return; }
        if (e.target.closest('#t16-modtoggle')) { var bd = $('t16-modbody'); bd.style.display = bd.style.display === 'none' ? 'block' : 'none'; return; }
        if (e.target.closest('#t16-modapply')) { BOARD.mods = Array.prototype.map.call(page.querySelectorAll('[data-mod]:checked'), function (c) { return c.getAttribute('data-mod'); }); board(sel.mode || 0); return; }
        if (e.target.closest('#t16-copyid')) { try { navigator.clipboard.writeText(String(S.id)); } catch (x) { } return; }
        var row = e.target.closest('tr[data-score]'); if (row && !e.target.closest('a')) location.href = '/scores/' + MODE_KEY[sel.mode || 0] + '/' + row.getAttribute('data-score');
      });
    }).catch(function (e) { page.innerHTML = '<div style="padding:30px;text-align:center">' + esc(e.message || 'Could not load this beatmapset.') + '<br><br><a href="/beatmapsets">Back to listing</a></div>'; });
  }

  function scorePage(id) {
    css('scores.css');
    json('/site/score?id=' + id).then(function (sc) {
      var bm = sc.beatmap || {}, m = sc.mode || 0, acc = sc.acc > 1.0001 ? sc.acc : sc.acc * 100;
      var mapHref = bm.beatmapset_id > 0 ? '/beatmapsets/' + bm.beatmapset_id + '#' + MODE_KEY[m] + '/' + bm.beatmap_id : '';
      var hit = function (cls, label, v) { return '<div class="stat-item ' + cls + '"><span class="hit-label">' + label + '</span><span class="hit-count">' + num(v) + 'x</span></div>'; };
      var rows = '<div class="stat-row">' + hit('hit-300', '300', sc.n300) + hit('hit-100', '100', sc.n100) + '</div><div class="stat-row">' + hit('hit-50', '50', sc.n50) + hit('hit-miss', 'Miss', sc.miss) + '</div>' + (m === 1 || m === 3 ? '<div class="stat-row">' + hit('hit-geki', 'Geki', sc.geki) + hit('hit-katu', 'Katu', sc.katu) + '</div>' : '');
      var mods = modsShort(sc.mods);
      page.innerHTML = '<div class="score-container"><div class="top-section"><div class="player-card"><div class="player-header"><a href="/users/' + encodeURIComponent(sc.player) + '"><img src="/site/avatar?name=' + encodeURIComponent(sc.player) + '" alt="" class="player-avatar avatar-frame"></a><div class="player-info"><span class="player-label"><a href="/users/' + encodeURIComponent(sc.player) + '" class="player-name">' + esc(sc.player) + '</a> ' + flag(sc.country) + '</span><div class="player-stats" id="t16-pstats"></div><div class="play-date">Played ' + esc(ago(sc.time)) + '</div></div></div></div>' +
        '<div class="beatmap-card"><div class="card-content"><div class="beatmap-info"><a href="' + (mapHref || '#') + '" class="beatmap-title"><b>' + esc(bm.artist) + ' - ' + esc(bm.title) + '</b> <span class="beatmap-version">[' + esc(bm.version) + ']</span> <span class="beatmap-stars" id="t16-stars"></span></a><br><a href="/beatmapsets?q=' + encodeURIComponent(bm.creator || '') + '" class="beatmap-creator">mapped by <span>' + esc(bm.creator) + '</span></a></div>' +
        (bm.beatmapset_id > 0 ? '<div class="beatmap-thumbnail"><a href="' + mapHref + '"><img src="https://assets.ppy.sh/beatmaps/' + bm.beatmapset_id + '/covers/card.jpg" alt="" onerror="this.style.opacity=0"></a></div>' : '') + '</div></div></div>' +
        '<div class="bottom-section"><div class="stats-panel"><div class="stats-header"><div class="score-display"><span class="score-label">Score:</span> <span class="score-value">' + num(sc.score) + '</span> &nbsp;&nbsp; <span class="score-label">Rank:</span> <span class="score-value">' + (sc.rank ? '#' + num(sc.rank) : 'N/A') + '</span></div></div><div class="stats-grid">' + rows + '</div>' +
        '<div class="score-metadata' + (mods !== 'None' ? ' has-mods' : '') + '"><div class="metadata-item combo"><span class="label">Max Combo</span><span class="value">' + num(sc.combo) + 'x</span></div><div class="metadata-item accuracy"><span class="label">Accuracy</span><span class="value">' + fix(acc, 2) + '%</span></div>' + (mods !== 'None' ? '<div class="metadata-item mods"><span class="label">Mods</span><span class="value">+' + esc(mods) + '</span></div>' : '') + '<div class="metadata-item pp"><span class="label">PP</span><span class="value" title="' + sc.pp + 'pp">' + (sc.pp > 0 ? num(Math.round(sc.pp)) + 'pp' : '-') + '</span></div></div></div>' +
        '<div class="grade-section"><div class="grade-wrapper"><img src="' + grade(sc.grade) + '" alt="Grade ' + esc(sc.grade) + '" class="grade-image"></div><div class="score-buttons">' + (mapHref ? '<a href="' + mapHref + '" class="score-button button-grey"><i class="icon-arrow-left"></i> Return to Beatmap</a>' : '') + '</div></div></div></div><br>';
      document.title = sc.player + ' on ' + bm.artist + ' - ' + bm.title;
      if (bm.beatmapset_id > 0) json('/site/beatmapset?id=' + bm.beatmapset_id).then(function (d) { var b = d.set && (d.set.beatmaps || []).filter(function (x) { return Number(x.id) === Number(bm.beatmap_id); })[0]; var el = $('t16-stars'); if (b && el && b.stars) el.innerHTML = '(' + fix(b.stars, 2) + ' <i class="icon-star"></i>)'; }).catch(function () { });
      json('/site/profile?name=' + encodeURIComponent(sc.player) + '&mode=' + MODE_KEY[m]).then(function (p) { var el = $('t16-pstats'); var r = p.global_rank || p.rank; if (el && r) el.innerHTML = '<span class="player-rank">Global Rank: <b>#' + num(r) + '</b>' + (p.pp ? ' (<span class="player-pp">' + num(Math.round(p.pp)) + 'pp</span>)' : '') + '</span>'; }).catch(function () { });
    }).catch(function (e) { page.innerHTML = '<div style="padding:30px;text-align:center">' + esc(e.message || 'Could not load this score.') + '<br><br><a href="/">Back to home</a></div>'; });
  }

  function friends() {
    document.title = 'Friends';
    page.innerHTML = '<h1 class="header-text" style="margin-top:10px">Friends</h1><div style="padding:0 10px" id="t16-fr">Loading...</div>';
    if (!session.user) { $('t16-fr').innerHTML = 'Please <a href="/profile#login">log in</a> to see your friends.'; return; }
    json('/site/friends').then(function (d) {
      var list = (d.friends || []).slice().sort(function (a, b) { return (b.online === true) - (a.online === true) || String(a.name).toLowerCase().localeCompare(String(b.name).toLowerCase()); });
      if (!list.length) { $('t16-fr').innerHTML = 'Your friends list is empty. Add people in-game with <b>!friend add &lt;username&gt;</b>.'; return; }
      $('t16-fr').innerHTML = '<table class="styled-table" style="width:100%"><thead><tr><th></th><th style="text-align:left">Player</th><th>Status</th></tr></thead><tbody>' + list.map(function (f, i) {
        var href = f.local ? '/users/' + encodeURIComponent(f.name) : '/users/' + f.id;
        return '<tr class="' + (i % 2 ? 'row-light' : 'row-dark') + '"><td style="width:40px"><img src="https://a.ppy.sh/' + f.id + '" width="32" height="32" alt="" onerror="this.style.visibility=\'hidden\'"></td><td>' + flag(f.country) + '<a href="' + href + '"' + (f.local ? '' : ' target="_blank" rel="noreferrer"') + '>' + esc(f.name) + '</a></td><td style="text-align:center">' + (f.online === true ? '<b style="color:#5a9c00">Online</b>' : (f.last_visit ? 'Last seen ' + esc(ago(f.last_visit)) : 'Offline')) + '</td></tr>';
      }).join('') + '</tbody></table>';
    }).catch(function (e) { $('t16-fr').textContent = e.message || 'Could not load friends.'; });
  }

  function boot() {
    var path = location.pathname.replace(/\/+$/, '');
    json('/site/session').then(function (s) { session.user = s && s.user || null; chrome(s || {}); }).catch(function () { chrome({}); }).then(function () {
      var sm = path.match(/\/scores\/(?:[a-z]+\/)?(\d+)$/), m = path.match(/\/beatmapsets\/(\d+)/), bm = path.match(/^(?:\/osu)?\/beatmaps\/(\d+)$/);
      if (document.body.classList.contains('t16-friends')) friends();
      else if (sm) scorePage(sm[1]); else if (m) setPage(m[1]); else if (bm) setPage(0, bm[1]); else listing();
    });
  }
  boot();
})();
