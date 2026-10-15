(function () {
  const TH = '/site/static/themes/2016/';
  const sel = (id) => document.getElementById(id);
  const open16 = { general: true };
  let lim = { act: 10, top: 10, most: 10, first: 10 };
  const gradeFile = (g) => ({ SS: 'X', SSH: 'XH' }[g] || g);
  const modText = (value) => {
    let mods = String(value || '').match(/.{1,2}/g) || [];
    mods = [...new Set(mods)].filter((m) => m !== 'NM' && !(m === 'DT' && mods.includes('NC')) && !(m === 'SD' && mods.includes('PF')));
    return mods.join('');
  };
  const mapLink = (b) => (Number(b.beatmap_id) > 0 ? `href="/beatmaps/${Number(b.beatmap_id)}"` : '');
  const mapName = (b) => `${esc(b.artist || 'Unknown artist')} - ${esc(b.title || 'Unknown beatmap')} [${esc(b.version || '?')}]`;
  const ago = (ts) => relative(ts);
  const modeNames = { osu: 'osu!', taiko: 'Taiko', fruits: 'CatchTheBeat', mania: 'osu!mania' };

  function renderLogin() {
    const box = sel('t16-login'); if (!box || !state.session) return;
    const user = state.session.user;
    if (user) {
      box.innerHTML = `<span class="nav-login">Welcome, <b><a href="/users/${encodeURIComponent(user)}">${esc(user)}</a></b></span>
        <img class="small-avatar" src="/site/avatar?name=${encodeURIComponent(user)}&v=${state.avatarVersion || AVATAR_BOOT}" alt="">
        <a href="#" id="t16-settings"><i class="icon-gear" style="color:#808080"></i> Settings</a>
        <a href="#" id="t16-switch"><i class="icon-exchange" style="color:#808080"></i> Switch profile</a>
        <a href="#" id="t16-logout"><i class="icon-power-off" style="color:#808080"></i> Logout</a>`;
      sel('t16-settings').onclick = (e) => { e.preventDefault(); openSettings(); };
      sel('t16-switch').onclick = (e) => { e.preventDefault(); openLogin(); };
      sel('t16-logout').onclick = (e) => { e.preventDefault(); sel('logout').click(); };
    } else {
      box.innerHTML = `<span class="nav-login">Welcome, guest!</span> <a href="#" id="t16-login-link"><i class="icon-caret-down"></i> Login</a>`;
      sel('t16-login-link').onclick = (e) => { e.preventDefault(); openLogin(); };
    }
    const p = state.profile;
    const n = (state.session.profiles || []).length;
    sel('t16-stats').innerHTML = `<b>${num(n)}</b> local profile${n === 1 ? '' : 's'}, <b>${state.session.active ? 1 : 0}</b> online now.<br>A total of <b>${num(p ? p.playcount : 0)}</b> plays by ${esc(p ? p.name : 'this player')}!`;
  }
  sel('t16-user-search')?.addEventListener('submit', (e) => {
    e.preventDefault(); const v = sel('user-search').value.trim(); if (v) location.href = `/users/${encodeURIComponent(v)}?mode=${state.mode}`;
  });
  sel('t16-map-search')?.addEventListener('submit', (e) => {
    e.preventDefault(); const v = sel('beatmap-search').value.trim(); if (v) location.href = `/beatmapsets?q=${encodeURIComponent(v)}`;
  });

  function hasNv() { return typeof nv !== 'undefined' && nv.addGraph && typeof d3 !== 'undefined'; }
  function drawRankGraph(p) {
    const svg = document.querySelector('#rank-graph svg'); if (!svg || !hasNv()) return;
    while (svg.firstChild) svg.removeChild(svg.firstChild);
    const today = new Date(); today.setHours(0, 0, 0, 0);
    const best = new Map();
    for (const [date, rank] of p.rank_history || []) {
      const d = new Date(date + 'T00:00:00'); if (isNaN(d)) continue;
      const x = -Math.max(0, Math.round((today - d) / 86400000));
      if (!rank) continue;
      if (!best.has(x) || rank < best.get(x)) best.set(x, rank);
    }
    if (p.global_rank) best.set(0, p.global_rank);
    const values = [...best.entries()].sort((a, b) => a[0] - b[0]).map(([x, r]) => ({ x, y: -r }));
    if (!values.length) { svg.parentNode.style.display = 'none'; return; }
    svg.parentNode.style.display = '';
    const ranks = values.map((v) => -v.y); const range = [Math.min(...ranks), Math.max(...ranks)];
    nv.addGraph(function () {
      const chart = nv.models.lineChart().margin({ left: 80, bottom: 20, right: 50 }).useInteractiveGuideline(true)
        .transitionDuration(250).interpolate('linear').showLegend(true).showYAxis(true).showXAxis(true);
      chart.xAxis.axisLabel('Days').tickFormat((d) => (d == 0 ? 'now' : d > 0 ? 'In ' + d + ' day' + (d != 1 ? 's' : '') : -d + ' day' + (d != -1 ? 's' : '') + ' ago'));
      chart.yAxis.axisLabel('Rank').tickFormat((r) => { r = Math.round(r); return r >= 0 ? '' : '#' + -r; });
      const digits = range[1].toString().length - 1;
      const minUnit = Math.pow(10, Math.max(digits, 0)), maxUnit = minUnit;
      const relMin = Math.round(range[0] / minUnit) * minUnit, relMax = Math.max(Math.round(range[1] / maxUnit) * maxUnit, relMin + 1);
      chart.yScale(d3.scale.linear().domain([-relMin - 1, -relMax]));
      chart.xAxis.tickValues([-90, -60, -30, 0]);
      chart.yAxis.tickValues([-relMax, -(relMax - relMax / 2), -relMin]);
      chart.forceY([-range[1] + 1, -range[0] - 1]);
      d3.select('#rank-graph svg').datum([{ values, key: 'Global Rank', color: '#ff7f0e' }]).call(chart);
      nv.utils.windowResize(chart.update);
      return chart;
    });
  }
  function drawPlayGraph(p) {
    const svg = document.querySelector('#play-graph svg'); if (!svg || !hasNv()) return;
    while (svg.firstChild) svg.removeChild(svg.firstChild);
    const now = new Date(), cy = now.getFullYear(), cm = now.getMonth();
    const values = (p.play_history || []).map(([m, n]) => {
      const [y, mo] = m.split('-').map(Number); return { x: -((cy - y) * 12 + (cm - (mo - 1))), y: n };
    }).sort((a, b) => a.x - b.x);
    if (!values.length) { svg.parentNode.style.display = 'none'; return; }
    svg.parentNode.style.display = '';
    const max = Math.max(...values.map((v) => v.y), 1); const step = Math.pow(10, Math.floor(Math.log10(max)));
    const top = Math.ceil(max / step) * step;
    nv.addGraph(function () {
      const chart = nv.models.lineChart().margin({ left: 80, bottom: 20, right: 50 }).useInteractiveGuideline(true)
        .transitionDuration(250).interpolate('linear').showLegend(false).showYAxis(true).showXAxis(true);
      chart.xAxis.axisLabel('Months').tickFormat((x) => (x == 0 ? 'now' : -x + ' month' + (x != -1 ? 's' : '') + ' ago'));
      chart.yAxis.axisLabel('Plays').tickFormat((y) => (Number.isInteger(y) ? String(y) : ''));
      const xs = values.map((v) => v.x), k = Math.ceil(xs.length / 7);
      chart.xAxis.tickValues(xs.filter((_, i) => (xs.length - 1 - i) % k === 0));
      chart.forceY([0, top]);
      d3.select('#play-graph svg').datum([{ values, key: 'Plays', color: '#f5f242', area: true }]).call(chart);
      nv.utils.windowResize(chart.update);
      return chart;
    });
  }

  function generalTab(p) {
    const own = state.session && state.session.user === p.name;
    if (!p.playcount && !p.pp && !p.ranked_score) {
      return `<div class="tab-spacing"><h3 class="profile-stats-header">No information recorded</h3>This user has not yet played any ranked maps in ${esc(modeNames[state.mode === 'rx' || state.mode === 'ap' || state.mode === 'vn' ? 'osu' : state.mode] || 'osu!')} mode.</div>`;
    }
    const lvl = scoreLevel(p.total_score);
    const cc = (p.details?.country || '').toLowerCase();
    const peak = p.rank_highest ? ` title="Peaked at #${num(p.rank_highest.rank)}"` : '';
    const era = p.rank_era && p.rank_era !== 'latest' ? ` <small title="Rank on the ${esc(p.rank_era_date || p.rank_era)} ladder">[${esc(p.rank_era)} ladder]</small>` : '';
    const acts = (p.recent || []).slice(0, lim.act).map((s) => `<tr><td style="width:20%">${ago(s.time)}</td><td><a href="/users/${encodeURIComponent(p.name)}?mode=${state.mode}">${esc(p.name)}</a> achieved a <b>${num(s.pp)}pp</b> play on <a ${mapLink(s.beatmap)}>${mapName(s.beatmap)}</a> (${esc(modeNames[state.mode] || 'osu!')}) rank ${esc(gradeFile(s.grade))}</td></tr>`).join('')
      || `<tr><td>This user hasn't done anything notable recently!</td></tr>`;
    const more = (p.recent || []).length > lim.act ? `<tr class="activity-more-row"><td colspan="2"><a href="javascript:void(0)" data-t16="more-act">Show me more...</a></td></tr>` : '';
    const g = p.grades || {};
    const gc = (k) => num(g[k] || 0);
    const gimg = (k, h = 42) => `<td style="width:42px"><img src="${TH}images/grades/${k}.png" height="${h}" alt="${k}"></td>`;
    return `<div class="tab-spacing">
      <div class="profile-performance"><h2><strong${peak}><a href="https://osu.ppy.sh/wiki/Performance_Points" target="_blank" rel="noreferrer">Performance</a>: <span title="${Number(p.pp_exact)>0?Number(p.pp_exact).toLocaleString(undefined,{minimumFractionDigits:2,maximumFractionDigits:4})+'pp':''}">${num(p.pp)}pp</span> ${p.global_rank ? `(#${num(p.global_rank)})` : '(#-)'}</strong>${era}</h2>
        ${p.country_rank && cc ? `<a href="https://osu.ppy.sh/rankings/osu/performance?country=${cc.toUpperCase()}" target="_blank" rel="noreferrer"><span class="t16-flag f-${cc}" title="${cc.toUpperCase()}"></span></a> #${num(p.country_rank)}` : ''}</div>
      <div class="profile-graph" id="rank-graph"><svg></svg></div>
      <div class="profile-recent"><h3 class="profile-stats-header">Recent Activity</h3><table><tbody>${acts}${more}</tbody></table></div>
      <div class="profile-detailed-stats"><h3 class="profile-stats-header">Detailed Stats</h3>
        <h4 class="profile-stats-element" title="Ranked Score consists of all your highest passing scores you have achieved. If you beat a score, only the difference between them will be added."><b>Ranked Score</b>: ${num(p.ranked_score)}</h4><br>
        <h4 class="profile-stats-element" title="Accuracy is calculated as a pp-weighted average of every beatmap you have achieved a passing score in, with each difficulty counting separately."><b>Hit Accuracy</b>: ${num(p.acc, 2)}%</h4><br>
        <h4 class="profile-stats-element" title="Total number of ranked beatmap plays. This includes both passes and fails."><b>Play Count</b>: ${num(p.playcount)}</h4><br>
        <h4 class="profile-stats-element" title="Total points scored in all ranked beatmap plays. This includes both passes and fails."><b>Total Score</b>: ${num(p.total_score)}</h4><br>
        <h4 class="profile-stats-element" title="Your level is based on your total accumulated score. You will gradually increase your level just by playing the game!"><b>Current Level</b>: ${lvl.level}</h4><br>
        <table class="level-bar"><tbody><tr><td class="level-bar-percent" style="width:${Math.round(lvl.progress * 2.96)}px">${lvl.progress}%</td><td style="background-color:white"></td></tr></tbody></table><br>
        <h4 class="profile-stats-element" title="Total notes hit."><b>Total Hits</b>: ${num(p.total_hits)}</h4><br>
        <h4 class="profile-stats-element" title="Highest combo achieved (hits in a row without a miss)."><b>Maximum Combo</b>: ${num(p.max_combo)}</h4><br>
        <h4 class="profile-stats-element"><b>Ranks</b></h4>
        <table class="profile-centered-space"><tbody>
          <tr>${gimg('XH')}<td style="width:50px">${gc('XH')}</td>${gimg('X')}<td style="width:50px">${gc('X')}</td>${gimg('SH')}<td style="width:50px">${gc('SH')}</td>${gimg('S')}<td style="width:50px">${gc('S')}</td></tr>
          <tr>${gimg('A')}<td style="width:50px">${gc('A')}</td>${gimg('B')}<td style="width:50px">${gc('B')}</td>${gimg('C')}<td style="width:50px">${gc('C')}</td>${gimg('D', 50)}<td style="width:50px">${gc('D')}</td></tr>
        </tbody></table></div></div>`;
  }
  function scoreEntry(p, s, i, weighted, own) {
    const mods = modText(s.mods);
    const w = Math.pow(0.95, i);
    return `<div class="score"><table><tbody><tr>
      <td class="score-left"><div><img src="${TH}images/grades/${gradeFile(s.grade)}_small.png" alt="${esc(s.grade)}"> <b><a ${mapLink(s.beatmap)}>${mapName(s.beatmap)}</a>${mods ? ` <b>+${esc(mods)}</b>` : ''}</b> (${num(s.acc, 2)}%)</div>
        <div class="score-bottom">${ago(s.time)}</div></td>
      <td class="score-right"><div class="pp-display"><b>${num(s.pp)}pp</b></div>
        <div class="pp-display-weight">${weighted ? `weighted <b>${Math.round(w * 100)}%</b> (${num(s.pp * w)}pp)` : ''}${own ? ` <a href="javascript:void(0)" data-delete-score="${esc(s.id)}" title="Delete score">delete</a>` : ''}</div></td>
    </tr></tbody></table></div>`;
  }
  function topTab(p, own) {
    const list = (p.top || []).slice(0, lim.top).map((s, i) => scoreEntry(p, s, i, true, own)).join('') || '<p>No awesome performance records yet :(</p>';
    const more = (p.top || []).length > lim.top ? `<div class="score show-more"><a href="javascript:void(0)" data-t16="more-top"><b>Show me more!</b></a></div>` : '';
    const fp = p.first_places || [];
    const fList = fp.slice(0, lim.first).map((s, i) => scoreEntry(p, s, i, false, own)).join('');
    const fMore = fp.length > lim.first ? `<div class="score show-more"><a href="javascript:void(0)" data-t16="more-first"><b>Show me more!</b></a></div>` : '';
    const firsts = fp.length ? `<div id="first-scores"><h2>First Place Ranks (${num(fp.length)})</h2>${fList}${fMore}</div>` : '';
    return `<div class="tab-spacing"><div id="top-scores"><h2>Best Performance${p.top?.length ? ` (${num(p.top.length)})` : ''}</h2>${list}${more}</div>${firsts}</div>`;
  }
  function historyTab(p) {
    const mp = (p.most_played || []).slice(0, lim.most).map((m, i) => `<div style="font-size:${Math.round(180 * Math.pow(0.95, i))}%;margin:2.5px">${num(m.count)} plays - <a ${mapLink(m.beatmap)}>${mapName(m.beatmap)}</a></div>`).join('') || '<p>This player has not played anything yet :(</p>';
    const moreMp = (p.most_played || []).length > lim.most ? `<div style="margin:2.5px"><a href="javascript:void(0)" data-t16="more-most">Show me more...</a></div>` : '';
    const rec = (p.recent_24h || []).map((s) => `<div style="margin:2.5px">${ago(s.time)} - <a ${mapLink(s.beatmap)}>${mapName(s.beatmap)}</a> ${num(s.pp)}pp (${esc(gradeFile(s.grade))}) ${modText(s.mods) ? '+' + esc(modText(s.mods)) : ''}</div>`).join('') || '<p>No recent scores set by this player :(</p>';
    return `<div class="tab-spacing">
      <div class="play-history"><h2 class="profile-stats-header">Play History</h2><div class="profile-graph" id="play-graph"><svg></svg></div></div>
      <div class="most-played"><h2 class="profile-stats-header">Most Played Beatmaps</h2><div id="plays-container">${mp}${moreMp}</div></div>
      <div class="recent-plays"><h2 class="profile-stats-header">Recent Plays</h2><div id="recent-container">${rec}</div></div></div>`;
  }
  const T16_ACH = [
    ['Beatmap Packs', [
      ['Anime Pack vol.1', 'anime1', 'all-packs-anime-1'], ['Anime Pack vol.2', 'anime2', 'all-packs-anime-2'], ['Anime Pack vol.3', 'anime3', 'all-packs-anime-3'], ['Anime Pack vol.4', 'anime4', 'all-packs-anime-4'],
      ['Internet! Pack vol.1', 'lulz1', 'all-packs-internet-1'], ['Internet! Pack vol.2', 'lulz2', 'all-packs-internet-2'], ['Internet! Pack vol.3', 'lulz3', 'all-packs-internet-3'], ['Internet! Pack vol.4', 'lulz4', 'all-packs-internet-4'],
      ['Rhythm Game Pack vol.1', 'rhythm1', 'all-packs-rhythm-1'], ['Rhythm Game Pack vol.2', 'rhythm2', 'all-packs-rhythm-2'], ['Rhythm Game Pack vol.3', 'rhythm3', 'all-packs-rhythm-3'], ['Rhythm Game Pack vol.4', 'rhythm4', 'all-packs-rhythm-4'],
      ['Video Game Pack vol.1', 'gamer1', 'all-packs-gamer-1'], ['Video Game Pack vol.2', 'gamer2', 'all-packs-gamer-2'], ['Video Game Pack vol.3', 'gamer3', 'all-packs-gamer-3'], ['Video Game Pack vol.4', 'gamer4', 'all-packs-gamer-4']]],
    ['Skill', [
      ['500 Combo (any song)', 'combo500', 'osu-combo-500'], ['750 Combo (any song)', 'combo750', 'osu-combo-750'], ['1000 Combo (any song)', 'combo1000', 'osu-combo-1000'], ['2000 Combo (any song)', 'combo2000', 'osu-combo-2000'],
      ['I can see the top', 'high-ranker-1', 'all-skill-highranker-1'], ['The gradual rise', 'high-ranker-2', 'all-skill-highranker-2'], ['Scaling up', 'high-ranker-3', 'all-skill-highranker-3'], ['Approaching the summit', 'high-ranker-4', 'all-skill-highranker-4']]],
    ['Dedication', [
      ['5,000 Plays (osu! mode)', 'plays1', 'osu-plays-5000'], ['15,000 Plays (osu! mode)', 'plays2', 'osu-plays-15000'], ['25,000 Plays (osu! mode)', 'plays3', 'osu-plays-25000'], ['50,000 Plays (osu! mode)', 'plays4', 'osu-plays-50000'],
      ['30,000 Drum Hits', 'taiko1', 'taiko-hits-30000'], ['300,000 Drum Hits', 'taiko2', 'taiko-hits-300000'], ['3,000,000 Drum Hits', 'taiko3', 'taiko-hits-3000000'],
      ['Catch 20,000 fruits', 'fruitsalad', 'fruits-hits-20000'], ['Catch 200,000 fruits', 'fruitplatter', 'fruits-hits-200000'], ['Catch 2,000,000 fruits', 'fruitod', 'fruits-hits-2000000'],
      ['40,000 Keys', 'maniahits1', 'mania-hits-40000'], ['400,000 Keys', 'maniahits2', 'mania-hits-400000'], ['4,000,000 Keys', 'maniahits3', 'mania-hits-4000000']]],
    ['Hush-Hush', [
      ['A meganekko approaches', 'meganekko', 'mania-secret-meganekko'], ['Challenge Accepted', 'challengeaccepted', 'all-secret-challenge_accepted'], ['Consolation Prize', 'consolationprize', 'all-secret-consolation_prize'],
      ["Don't let the bunny distract you!", 'bunny', 'all-secret-bunny'], ['Jack of All Trades', 'jack', 'all-secret-jack'], ['Jackpot', 'jackpot', 'all-secret-jackpot'], ['Most Improved', 'improved', 'all-secret-improved'],
      ['Non-stop Dancer', 'dancer', 'all-secret-dancer'], ['Nonstop', 'nonstop', 'all-secret-nonstop'], ['Obsessed', 'obsessed', 'all-secret-obsessed'], ['Quick Draw', 'quickdraw', 'all-secret-quick_draw'],
      ['S-Ranker', 's-ranker', 'all-secret-rank-s'], ['Stumbler', 'stumbler', 'all-secret-stumbler']]],
  ];
  const ACH_DIR = '/site/static/themes/2016/images/achievements/';
  async function achievementsTab(p) {
    const cat = await loadMedalsCatalog();
    const bySlug = new Map(cat.map((m) => [m.icon_url, m]));
    const have = new Map((p.medals || []).map((m) => [m.id, m]));
    let n = 0, h = '<div class="tab-spacing">';
    for (const [name, list] of T16_ACH) {
      h += `<h2 class="profile-achievement-header">${esc(name)}</h2><div class="achievement-category">` + list.map(([title, file, slug]) => {
        const def = bySlug.get(slug); const got = def && have.get(def.id);
        if (got) { n++; const when = got.achieved_at ? new Date(got.achieved_at * 1000).toISOString().replace('T', ' ').slice(0, 19) + ' UTC' : ''; return `<div class="achievement"><img src="${ACH_DIR}${file}.png" title="Unlocked ${when}" alt="Achievement Image" loading="lazy"><br><b>${esc(title)}</b></div>`; }
        return `<div class="achievement"><img src="${ACH_DIR}unknown.png" alt="Unknown Achievement" loading="lazy"><br><b>${esc(title)}</b></div>`;
      }).join('') + '</div>';
    }
    return h + '</div>';
  }

  function details(p, own) {
    const d = p.details || {}; const rows = [];
    const row = (icon, title, inner) => `<div title="${title}"><i class="${icon}"></i><div>${inner}</div></div>`;
    if (d.location) rows.push(row('icon-map-marker', 'Location', esc(d.location)));
    if (d.interests) rows.push(row('icon-heart-empty', 'Interests', esc(d.interests)));
    if (d.occupation) rows.push(row('icon-briefcase', 'Occupation', esc(d.occupation)));
    rows.push('<div></div>');
    const url = (v) => (/^https?:\/\//i.test(v) ? v : 'https://' + v);
    if (d.website) rows.push(row('icon-globe', 'Website', `<a href="${esc(url(d.website))}" target="_blank" rel="noreferrer">${esc(d.website)}</a>`));
    if (d.twitter) rows.push(row('icon-twitter', 'Twitter', `<a href="https://twitter.com/${esc(String(d.twitter).replace(/^@/, ''))}" target="_blank" rel="noreferrer">${esc(d.twitter)}</a>`));
    if (d.discord) rows.push(row('icon-discord', 'Discord', esc(d.discord)));
    rows.push('<div></div>');
    let arrivedTs = null, arrivedTitle = '';
    if (d.join_date) { const t = new Date(d.join_date + '-01T12:00:00'); if (!isNaN(t)) { arrivedTs = t.getTime() / 1000; arrivedTitle = t.toLocaleDateString('en-US', { month: 'long', year: 'numeric' }); } }
    if (!arrivedTs && p.first_play) { arrivedTs = p.first_play; arrivedTitle = new Date(p.first_play * 1000).toISOString().slice(0, 10); }
    rows.push(row('icon-signin', 'Arrived', `<span title="${esc(arrivedTitle)}">${arrivedTs ? esc(relative(arrivedTs)) : 'today'}</span>`));
    rows.push(p.active ? `<div title="Last Active"><i class="icon-signout online"></i><div>Online</div></div>` : row('icon-signout', 'Last Active', p.last_play ? esc(relative(p.last_play)) : 'dead'));
    return `<div class="profile-details">${rows.join('')}${own ? '<a class="edit-profile" href="#" data-t16="edit">Edit</a>' : ''}</div>`;
  }
  async function render() {
    const p = state.profile; if (!p) return;
    const own = !!(state.session && state.session.user === p.name);
    document.title = `${p.name} - Player Info`;
    const d = p.details || {}; const cc = (d.country || '').toLowerCase();
    const devs = (d.devices || []).map((x) => x.toLowerCase());
    const ps = (id, cls, key) => `<div id="${id}" class="playstyle ${cls}${devs.includes(key) ? ' playstyle-using' : ''}"></div>`;
    const about = d.about && d.about.trim() ? `<div class="userpage"><div class="bbcode">${parseBBCode(d.about)}</div></div>` : '';
    const edit = own ? (about ? `<a class="edit-button" href="#" data-t16="edit">Edit</a><br>` : `<a href="#" data-t16="edit">Click to make your user page!</a><br>`) : '';
    const rs = ['osu', 'vn', 'rx', 'ap'].includes(state.mode) ? 'osu' : state.mode;
    const osuLabel = state.mode === 'rx' ? 'osu!<small> (Relax)</small>' : state.mode === 'ap' ? 'osu!<small> (Autopilot)</small>' : 'osu!';
    const gm = (m, label) => `<a class="gamemode-button${rs === m ? ' active-mode' : ''}" href="#" data-t16-mode="${m}">${label}</a>`;
    const tabs = [['general', 'General'], ['leader', 'Top Plays'], ['history', 'Historical'], ['beatmaps', 'Beatmaps'], ['kudosu', 'Kudosu!'], ['achievements', 'Achievements']];
    const body = {
      general: generalTab(p), leader: topTab(p, own), history: historyTab(p),
      beatmaps: '<div class="tab-spacing"><p>Beatmap ownership and favourites are not recorded by this local server.</p></div>',
      kudosu: '<div class="tab-spacing"><p>This user has not earned any kudosu yet.</p></div>',
      achievements: await achievementsTab(p),
    };
    const tabHtml = tabs.map(([id, name]) => `<tr data-t16-tab="${id}"><td id="tab-${id}" class="tab-heading">${name}</td></tr>
      <tr><td><div id="${id}" class="tab-content${open16[id] ? ' expanded' : ''}" style="display:${open16[id] ? 'block' : 'none'}">${body[id]}</div></td></tr>`).join('');
    sel('profile-content').innerHTML = `
      <div class="profile-left">
        <div class="profile-user-card"><div class="avatar avatar-frame"><img id="avatar" alt="" width="128" height="128"></div>
          <div class="profile-username-container"><div class="profile-username" style="color:#000">${esc(p.name)}</div></div>
          <div style="padding:3px"><center>${cc.length === 2 ? `<a href="https://osu.ppy.sh/rankings/osu/performance?country=${cc.toUpperCase()}" target="_blank" rel="noreferrer"><span class="t16-flag f-${cc}" title="${esc(countryName(cc.toUpperCase()) || cc.toUpperCase())}"></span></a>` : ''}</center></div></div>
        ${details(p, own)}
        <div class="playstyle-container">${ps('Mouse', 'playstyle-mouse', 'mouse')}${ps('Keyboard', 'playstyle-keyboard', 'keyboard')}${ps('Tablet', 'playstyle-tablet', 'tablet')}${ps('Touch', 'playstyle-touch', 'touchscreen')}</div>
      </div>
      <div class="profile-right">${edit}${about}
        <div class="gamemode-container">${gm('osu', osuLabel)}${gm('taiko', 'Taiko')}${gm('fruits', 'CatchTheBeat')}${gm('mania', 'osu!mania')}</div>
        <table class="profile-table"><tbody>
          <tr><td style="padding:0"><table class="profile-tabs"><tbody><tr>${tabs.map(([id, n]) => `<td data-t16-tabbtn="${id}">${n}</td>`).join('')}</tr></tbody></table></td></tr>
          ${tabHtml}</tbody></table></div><div style="clear:both"></div>`;
    const av = sel('avatar');
    av.src = `/site/avatar?name=${encodeURIComponent(p.name)}&v=${state.avatarVersion || AVATAR_BOOT}`;
    sel('welcome').hidden = true; sel('profile-content').hidden = false;
    renderLogin();
    drawAll(p);
  }
  function drawAll(p) { if (open16.general) drawRankGraph(p); if (open16.history) drawPlayGraph(p); }

  document.addEventListener('click', (e) => {
    const pc = e.target.closest('#profile-content'); if (!pc) return;
    const mode = e.target.closest('[data-t16-mode]');
    if (mode) {
      e.preventDefault();
      const clicked = mode.dataset.t16Mode; const cur = ['osu', 'vn', 'rx', 'ap'].includes(state.mode) ? 'osu' : state.mode;
      let target = clicked;
      if (clicked === 'osu' && cur === 'osu') target = state.mode === 'osu' || state.mode === 'vn' ? 'rx' : state.mode === 'rx' ? 'ap' : 'osu';
      else if (clicked !== 'osu' && state.mode === clicked) return;
      state.mode = target; lim = { act: 10, top: 10, most: 10, first: 10 };
      loadProfile(state.name, true);
      return;
    }
    const btn = e.target.closest('[data-t16-tabbtn],[data-t16-tab]');
    if (btn) {
      const id = btn.dataset.t16Tabbtn || btn.dataset.t16Tab;
      const el = sel(id); if (!el) return;
      const force = !!btn.dataset.t16Tabbtn;
      const show = force ? true : el.style.display === 'none';
      el.style.display = show ? 'block' : 'none'; el.classList.toggle('expanded', show); open16[id] = show;
      if (show) { if (id === 'general') drawRankGraph(state.profile); if (id === 'history') drawPlayGraph(state.profile); }
      if (force) sel('tab-' + id).scrollIntoView();
      return;
    }
    const act = e.target.closest('[data-t16]');
    if (act) {
      e.preventDefault();
      const k = act.dataset.t16;
      if (k === 'edit') openSettings();
      else { if (k === 'more-act') lim.act += 10; if (k === 'more-top') lim.top += 10; if (k === 'more-first') lim.first += 10; if (k === 'more-most') lim.most += 10; render(); }
    }
  });
  window.renderProfile16 = render;
  window.addEventListener('resize', () => { if (state.profile) {  } });
})();
