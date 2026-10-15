'use strict';
const $ = (id) => document.getElementById(id);
function readEra(){try{const v=localStorage.getItem('site_era');return /^\d{4}$/.test(v||'')?v:'latest';}catch(e){return 'latest';}}
function readPpSys(){try{const v=localStorage.getItem('site_ppsys');return /^[a-z0-9]{3,8}$/.test(v||'')?v:'live';}catch(e){return 'live';}}
function readPpFollow(){try{return localStorage.getItem('site_ppfollow')!=='0';}catch(e){return true;}}
const state = {era: readEra(), ppsys: readPpSys(), ppfollow: readPpFollow(), session: null, profile: null, name: null, mode: 'osu', topLimit: 5, firstLimit: 5, recentLimit: 5, actLimit: 5, request: 0};
const num = (n, digits = 0) => Number(n || 0).toLocaleString('en-US', {minimumFractionDigits: digits, maximumFractionDigits: digits});
(()=>{const t=new URLSearchParams(location.search).get('theme');if(t!==null){document.cookie=`site_theme=${encodeURIComponent(t)}; path=/; max-age=${t?31536000:0}; SameSite=Lax`;const u=new URL(location.href);u.searchParams.delete('theme');location.replace(u);}})();
const AVATAR_BOOT=Date.now();
const T19 = document.body.classList.contains('theme-2019');
const T16 = document.body.classList.contains('theme-2016');
const NEXT = document.body.classList.contains('theme-next') || T19;
const fixedSectionOrder = () => { const v = document.body.dataset.fixedSections; return v ? v.split(',') : null; };
const esc = (value) => String(value ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
function relative(timestamp) {
  if (!timestamp) return 'No recorded plays';
  const seconds = Math.max(0, Date.now() / 1000 - timestamp);
  for (const [unit, size] of [['year',31536000],['month',2592000],['day',86400],['hour',3600],['minute',60]]) {
    if (seconds >= size) { const n = Math.floor(seconds / size); return `${T19 && ['hour','month','year'].includes(unit) ? 'about ' : ''}${n} ${unit}${n === 1 ? '' : 's'} ago`; }
  }
  return 'just now';
}
async function api(path, body) {
  const options = body === undefined ? {} : {method:'POST',headers:{'Content-Type':'application/json','X-CSRF-Token':state.session.csrf},body:JSON.stringify(body)};
  const res = await fetch(path, options);
  let data;
  try { data = await res.json(); } catch { throw new Error('The local server returned an unexpected response. Restart it and refresh this page.'); }
  if (!res.ok) throw new Error(data.error || 'The request failed.');
  return data;
}
function showMessage(text) { $('page-message').textContent = text; $('page-message').hidden = !text; }
function renderSession() {
  $('account-name').textContent = state.session.user || 'Sign in';
  const accountImage=state.session.user?`/site/avatar?name=${encodeURIComponent(state.session.user)}&v=${state.avatarVersion||AVATAR_BOOT}`:fallbackAvatar;
  if($('account-avatar').getAttribute('src')!==accountImage)$('account-avatar').src=accountImage;
  $('account-panel-name').textContent=state.session.user||'Guest';
  $('account-panel-status').textContent=state.session.active===state.session.user?'Active on the local server':'Local osu!';
  $('account-profile').href=state.session.user?`/users/${encodeURIComponent(state.session.user)}${state.mode ? `?mode=${state.mode}` : ''}`:'#profile';
  $('account-settings').hidden=!state.session.user;
  $('account-logout').hidden=!state.session.user;
  $('account-profile').hidden=!state.session.user;
  $('profile-names').replaceChildren(...state.session.profiles.map(name => { const option = document.createElement('option'); option.value = name; return option; }));
  $('signed-in-actions').hidden = !state.session.user;
  $('signed-in-as').textContent = state.session.user ? `Signed in as ${state.session.user}` : '';
}
function openLogin() {
  closeAccount();
  if (window.SiteNav && state.session) { $('mobile-menu').hidden=true; $('mobile-menu-toggle').setAttribute('aria-expanded','false'); SiteNav.setSession(state.session); SiteNav.install(); SiteNav.openLogin(!!state.session.user); return; }
  $('login-error').hidden = true;
  $('login-username').value = state.session?.user || state.name || '';
  $('mobile-menu').hidden=true;
  $('mobile-menu-toggle').setAttribute('aria-expanded','false');
  $('login-title').textContent=state.session?.user?'Switch profile':'Sign in';
  $('login-dialog').showModal();
  $('login-username').focus();
  $('login-username').select();
}
function modIcons(value) {
  let mods = String(value || 'NM').match(/.{1,2}/g) || ['NM'];
  mods = [...new Set(mods)].filter(m => m !== 'NM' && !(m==='DT' && mods.includes('NC')) && !(m==='SD' && mods.includes('PF')));
  if (!mods.length) return '<div class="mods mods--none"></div>';
  return `<div class="mods">${mods.map(m=>{const type=['EZ','HT','NF'].includes(m)?'DifficultyReduction':['RX','AP','AT'].includes(m)?'Automation':m==='NM'?'System':'DifficultyIncrease';return `<div class="mod mod--type-${type}" title="${esc(m)}"><div class="mod__icon mod__icon--${esc(m)}" data-acronym="${esc(m)}"></div></div>`;}).join('')}</div>`;
}
function scoreRow19(score, index, top) {
  const map=score.beatmap||{};
  const link=Number(map.beatmap_id)>0?`/beatmaps/${Number(map.beatmap_id)}`:null;
  const weight=.95**index;
  const grade=`<div class="score-rank score-rank--full score-rank--${esc(score.grade)}" role="img" aria-label="Grade ${esc(score.grade.replace('X','SS'))}"></div>`;
  const key=`${top?'top':'recent'}-${index}`;
  const title=esc(map.title||'Unknown beatmap');
  const titleHtml=link?`<a class="nx-play__title" href="${link}" target="_blank" rel="noreferrer">${title}</a>`:`<span class="nx-play__title">${title}</span>`;
  const tops=(state.profile&&state.profile.top)||[];
  const rewards=top||(Number(score.pp)>0&&tops.some(t=>String(t.id)===String(score.id)));
  const wpp=top?` <span class="nx-play__wpp">${num(score.pp*weight)}pp</span>`:'';
  const wline=top?`<div class="nx-play__weight">weighted ${num(weight*100)}%</div>`:'';
  const ppCell=rewards?`<strong>${num(score.pp)}</strong><span class="nx-play__unit">pp</span>`:'<strong>-</strong>';
  return `<div class="nx-play" data-score="${esc(score.id)}"><div class="nx-play__top"><div class="nx-play__grade">${grade}</div><div class="nx-play__main">${titleHtml} <small class="nx-play__artist">by ${esc(map.artist||'unknown artist')}</small><div class="nx-play__sub"><span class="nx-play__diff">${esc(map.version||'Unknown difficulty')}</span><time class="nx-play__time">${relative(score.time)}</time></div></div></div><div class="nx-play__bottom"><div class="nx-play__mods">${modIcons(score.mods)}</div><div class="nx-play__acc"><div class="nx-play__accrow"><span class="nx-play__accv">${num(score.acc,2)}%</span>${wpp}</div>${wline}</div><div class="nx-play__pp19${rewards?'':' nx-play__pp19--none'}">${ppCell}</div></div><button class="popup-menu nx-play__more" data-score-detail="${key}" aria-label="Score details for ${esc(map.title||'unknown beatmap')}" aria-expanded="false" type="button"><i class="fas fa-ellipsis-v" aria-hidden="true"></i></button></div><div id="detail-${key}" class="score-expanded" hidden><span>Score <b>${num(score.score)}</b></span><span>Combo <b>${num(score.max_combo)}x</b></span><span>300 / 100 / 50 <b>${num(score.n300)} / ${num(score.n100)} / ${num(score.n50)}</b></span><span>Misses <b>${num(score.nmiss)}</b></span><span>Mods <b>${esc(score.mods)}</b></span>${state.session.user===state.name?`<button class="subtle-button" data-delete-score="${esc(score.id)}" type="button">Delete score</button>`:''}</div>`;
}
function scoreRowNext(score, index, top) {
  if (T19) return scoreRow19(score, index, top);
  const map=score.beatmap||{};
  const link=Number(map.beatmap_id)>0?`/beatmaps/${Number(map.beatmap_id)}`:null;
  const weight=.95**index;
  const grade=`<div class="score-rank score-rank--full score-rank--${esc(score.grade)}" role="img" aria-label="Grade ${esc(score.grade.replace('X','SS'))}"></div>`;
  const key=`${top?'top':'recent'}-${index}`;
  const title=`${esc(map.title||'Unknown beatmap')} [${esc(map.version||'Unknown difficulty')}]`;
  const titleHtml=link?`<a class="nx-play__title" href="${link}" target="_blank" rel="noreferrer">${title}</a>`:`<span class="nx-play__title">${title}</span>`;
  const weighted=top?`<small>weighted: ${num(score.pp*weight)}pp (${num(weight*100)}%)</small>`:'';
  const tops=(state.profile&&state.profile.top)||[];
  const rewards=top||(Number(score.pp)>0&&tops.some(t=>String(t.id)===String(score.id)));
  const ppBlock=rewards?`<strong>${num(score.pp)}pp</strong>${weighted}`:`<strong>${num(score.score)}</strong>`;
  return `<div class="nx-play" data-score="${esc(score.id)}"><div class="nx-play__grade">${grade}</div><div class="nx-play__main">${titleHtml} <small class="nx-play__artist">${esc(map.artist||'unknown artist')}</small><div class="nx-play__time">${relative(score.time)}</div></div><div class="nx-play__mods">${modIcons(score.mods)}</div><div class="nx-play__pp">${ppBlock}<small>accuracy: ${num(score.acc,2)}%</small></div><button class="popup-menu nx-play__more" data-score-detail="${key}" aria-label="Score details for ${esc(map.title||'unknown beatmap')}" aria-expanded="false" type="button"><i class="fas fa-ellipsis-v" aria-hidden="true"></i></button></div><div id="detail-${key}" class="score-expanded" hidden><span>Score <b>${num(score.score)}</b></span><span>Combo <b>${num(score.max_combo)}x</b></span><span>300 / 100 / 50 <b>${num(score.n300)} / ${num(score.n100)} / ${num(score.n50)}</b></span><span>Misses <b>${num(score.nmiss)}</b></span><span>Mods <b>${esc(score.mods)}</b></span>${state.session.user===state.name?`<button class="subtle-button" data-delete-score="${esc(score.id)}" type="button">Delete score</button>`:''}</div>`;
}
function scoreRow(score, index, top) {
  if (NEXT) return scoreRowNext(score, index, top);
  const map=score.beatmap||{};
  const link=Number(map.beatmap_id)>0?`/beatmaps/${Number(map.beatmap_id)}`:null;
  const weight=.95**index;
  const grade=`<div class="score-rank score-rank--full score-rank--${esc(score.grade)}" role="img" aria-label="Grade ${esc(score.grade.replace('X','SS'))}"></div>`;
  const key=`${top?'top':'recent'}-${index}`;
  return `<div class="play-detail play-detail--highlightable" data-score="${esc(score.id)}"><div class="play-detail__group play-detail__group--top"><div class="play-detail__icon play-detail__icon--main">${grade}</div><div class="play-detail__detail"><a class="play-detail__title u-ellipsis-overflow" ${link?`href="${link}" target="_blank" rel="noreferrer"`:''}>${esc(map.title||'Unknown beatmap')} <small class="play-detail__artist">by ${esc(map.artist||'unknown artist')}</small></a><div class="play-detail__beatmap-and-time"><span class="play-detail__beatmap">${esc(map.version||'Unknown difficulty')}</span><time class="play-detail__time">${relative(score.time)}</time></div></div></div><div class="play-detail__group play-detail__group--bottom"><div class="play-detail__score-detail"><div class="play-detail__icon play-detail__icon--extra">${grade}</div><div class="play-detail__score-detail-top-right"><div class="play-detail__accuracy-and-weighted-pp"><span class="play-detail__accuracy">${num(score.acc,2)}%</span>${top?`<span class="play-detail__weighted-pp">${num(score.pp*weight)}pp</span>`:''}</div>${top?`<div class="play-detail__pp-weight">weighted ${num(weight*100)}%</div>`:''}</div></div><div class="play-detail__mods-pp"><div class="play-detail__mods">${modIcons(score.mods)}</div><div class="play-detail__pp">${num(score.pp)}<span class="play-detail__pp-unit">pp</span></div></div><div class="play-detail__more"><button class="popup-menu" data-score-detail="${key}" aria-label="Score details for ${esc(map.title||'unknown beatmap')}" aria-expanded="false" type="button"><i class="fas fa-ellipsis-v" aria-hidden="true"></i></button></div></div></div><div id="detail-${key}" class="score-expanded" hidden><span>Score <b>${num(score.score)}</b></span><span>Combo <b>${num(score.max_combo)}x</b></span><span>300 / 100 / 50 <b>${num(score.n300)} / ${num(score.n100)} / ${num(score.n50)}</b></span><span>Misses <b>${num(score.nmiss)}</b></span><span>Mods <b>${esc(score.mods)}</b></span>${state.session.user===state.name?`<button class="subtle-button" data-delete-score="${esc(score.id)}" type="button">Delete score</button>`:''}</div>`;
}
function renderHistory() {
  const history=state.profile.play_history||[];
  if(!history.length){$('play-history').textContent='No play history yet.';return;}
  const start=new Date(history[0][0]+'-01T00:00:00Z');
  const end=new Date(history[history.length-1][0]+'-01T00:00:00Z');
  const values=new Map(history);
  const months=[];
  while(start<=end){const month=start.toISOString().slice(0,7);months.push([month,values.get(month)||0]);start.setUTCMonth(start.getUTCMonth()+1);}
  const data=months.slice(-240),max=Math.max(...data.map(x=>x[1]),1);
  const points=data.map(([m,n],i)=>`${45+i*535/Math.max(data.length-1,1)},${210-n/max*180}`).join(' ');
  const lines=[0,.25,.5,.75,1].map(f=>`<line x1="45" x2="580" y1="${210-f*180}" y2="${210-f*180}" stroke="currentColor" opacity=".15"/><text x="36" y="${214-f*180}" text-anchor="end">${num(max*f)}</text>`).join('');
  if(T19){
    const mon=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
    const W=860,L=84,R=800,T=30,B=210,n=data.length;
    const xs=i=>L+i*(R-L)/Math.max(n-1,1);
    const ys=v=>B-v/max*(B-T);
    const step=n>36?3:Math.max(1,Math.ceil(n/12));
    const pts=data.map(([m,v],i)=>`${xs(i).toFixed(1)},${ys(v).toFixed(1)}`).join(' ');
    const hl=[0,.25,.5,.75,1].map(f=>`<line x1="${L}" x2="${R}" y1="${ys(max*f)}" y2="${ys(max*f)}" stroke="#000"/><text x="${L-12}" y="${ys(max*f)+4}" text-anchor="end">${num(Math.round(max*f))}</text>`).join('');
    const grid=data.map((_,i)=>i%step===0?`<line x1="${xs(i)}" x2="${xs(i)}" y1="${T}" y2="${B}" stroke="#000"/>`:'').join('');
    const labels=data.map(([m],i)=>{if(i%step!==0)return '';const [y,mo]=m.split('-');return `<text transform="translate(${xs(i)+4} 222) rotate(-45)" text-anchor="end">${mon[(+mo||1)-1]} ${y}</text>`;}).join('');
    $('play-history').innerHTML=`<svg viewBox="0 0 ${W} 290" role="img" aria-label="Monthly play counts from saved scores">${hl}${grid}<polyline points="${pts}" fill="none" stroke="#ffcc22" stroke-width="2"/>${data.map(([m,v],i)=>`<circle cx="${xs(i)}" cy="${ys(v)}" r="${n>24?0:3}" fill="#ffcc22"><title>${m}: ${v} plays</title></circle>`).join('')}${labels}</svg>`;
    return;
  }
  $('play-history').innerHTML=`<svg viewBox="0 0 600 250" role="img" aria-label="Monthly play counts from saved scores">${lines}<polyline points="${points}" fill="none" stroke="#ffcc22" stroke-width="2"/>${data.map(([m,n],i)=>`<circle cx="${45+i*535/Math.max(data.length-1,1)}" cy="${210-n/max*180}" r="3" fill="#ffcc22"><title>${m}: ${n} plays</title></circle>`).join('')}<text x="45" y="238">${esc(data[0][0])}</text><text x="580" y="238" text-anchor="end">${data.length>1?esc(data[data.length-1][0]):''}</text></svg>`;
}
function renderMostPlayed() {
  const maps=state.profile.most_played||[];
  $('most-count').textContent=num(maps.length);
  $('most-played').innerHTML=maps.slice(0,state.mostLimit||5).map(({count,beatmap:m})=>{
    const countMarkup=`<span class="beatmap-playcount__count"><i class="fas fa-play beatmap-playcount__count-icon" aria-hidden="true"></i>${num(count)}</span>`;
    const mapId=Number(m.beatmap_id||0);
    const setId=Number(m.beatmapset_id||0);
    const link=mapId>0?`href="/beatmaps/${mapId}"`:'';
    const coverTag=link?'a':'div';
    const coverUrl=setId>0?`https://assets.ppy.sh/beatmaps/${setId}/covers/list.jpg`:'';
    const coverUrl2x=setId>0?`https://assets.ppy.sh/beatmaps/${setId}/covers/list@2x.jpg`:'';
    const coverStyle=setId>0?`style="background-image: url('${coverUrl2x}'), url('${coverUrl}');"`:'';
    const coverInner=setId>0?`<span class="beatmapset-cover beatmapset-cover--full" style="--bg: url('${coverUrl}'); --bg-2x: url('${coverUrl2x}'); background-image: url('${coverUrl2x}');"></span>`:'';
    if(NEXT){
      const cover=Number(m.beatmapset_id)>0?`style="background-image:url('https://assets.ppy.sh/beatmaps/${Number(m.beatmapset_id)}/covers/list.jpg')"`:'';
      return `<div class="nx-mp"><div class="nx-mp__thumb" ${cover}></div><div class="nx-mp__info"><a class="nx-mp__title" ${link}>${esc(m.title||'Unknown beatmap')} [${esc(m.version||'?')}]</a> <small class="nx-mp__artist">${T19?'by ':''}${esc(m.artist||'unknown artist')}</small>${m.creator?`<div class="nx-mp__mapper">mapped by <a>${esc(m.creator)}</a></div>`:''}</div><div class="nx-mp__count">times played <strong>${num(count)}</strong></div></div>`;
    }
    return `<div class="beatmap-playcount"><${coverTag} class="beatmap-playcount__cover" ${link} ${coverStyle}>${coverInner}<div class="beatmap-playcount__cover-count">${countMarkup}</div></${coverTag}><div class="beatmap-playcount__detail"><div class="beatmap-playcount__info"><div class="u-ellipsis-overflow"><a class="beatmap-playcount__title" ${link}>${esc(m.title||'Unknown beatmap')} <span class="beatmap-playcount__title-artist">by ${esc(m.artist||'unknown artist')}</span></a></div><div class="beatmap-playcount__info-row u-ellipsis-overflow"><span class="beatmap-playcount__artist">${esc(m.artist||'unknown artist')}</span><span>${esc(m.version||'?')}</span>${m.creator?` <span class="beatmap-playcount__mapper">mapped by <b>${esc(m.creator)}</b></span>`:''}</div></div><div class="beatmap-playcount__detail-count">${countMarkup}</div></div></div>`;
  }).join('')||'<p class="empty-inline">No plays recorded yet.</p>';
  $('more-most').hidden=maps.length<=(state.mostLimit||5);
}
function renderScores() {
  const p = state.profile;
  $('top-count').textContent = num(p.top.length);
  const recent=p.recent_24h||[];
  $('recent-count').textContent = num(recent.length);
  $('top-scores').innerHTML = p.top.length ? p.top.slice(0,state.topLimit).map((s,i)=>scoreRow(s,i,true)).join('') : '<p class="empty-inline">No ranked plays yet. Your best performances will appear here.</p>';
  $('recent-scores').innerHTML = recent.length ? recent.slice(0,state.recentLimit).map((s,i)=>scoreRow(s,i,false)).join('') : '<p class="empty-inline">No plays in the last 24 hours.</p>';
  $('more-top').hidden = p.top.length <= state.topLimit;
  const fp=p.first_places||[];
  $('firsts-block').hidden=!fp.length;
  $('firsts-count').textContent=num(fp.length);
  $('firsts-scores').innerHTML=fp.slice(0,state.firstLimit).map((s,i)=>scoreRow(s,i,false)).join('');
  $('more-firsts').hidden=fp.length<=state.firstLimit;
  $('more-recent').hidden = recent.length <= state.recentLimit;
}

async function loadMedalsCatalog() {
  if (state.medalsCatalog) return state.medalsCatalog;
  try {
    const res = await fetch('/site/medals');
    if (res.ok) {
      state.medalsCatalog = await res.json();
      return state.medalsCatalog;
    }
  } catch (e) {
    console.error('Failed to load medals catalog', e);
  }
  return [];
}

let medalTooltipEl = null;
function setupMedalTooltips() {
  if (!medalTooltipEl) {
    medalTooltipEl = document.createElement('div');
    medalTooltipEl.id = 'medal-tooltip';
    medalTooltipEl.className = 'medal-tooltip';
    medalTooltipEl.setAttribute('role', 'tooltip');
    medalTooltipEl.innerHTML = `
      <div id="medal-tooltip-category" class="medal-tooltip__header"></div>
      <div class="medal-tooltip__body">
        <div class="medal-tooltip__icon-wrapper">
          <img id="medal-tooltip-img" class="medal-tooltip__img" src="" alt="" />
        </div>
        <div id="medal-tooltip-name" class="medal-tooltip__name"></div>
        <div id="medal-tooltip-desc" class="medal-tooltip__desc"></div>
        <div class="medal-tooltip__footer">
          <div id="medal-tooltip-status" class="medal-tooltip__status"></div>
          <div id="medal-tooltip-date" class="medal-tooltip__date"></div>
        </div>
      </div>
      <div class="medal-tooltip__arrow"></div>
    `;
    document.body.appendChild(medalTooltipEl);
  }

  const container = $('medals-content');
  if (!container || container._tooltipsInitialized) return;
  container._tooltipsInitialized = true;

  const showTip = (badge) => {
    if (!badge || !badge.classList.contains('badge-achievement')) return;
    const name = badge.dataset.name;
    const desc = badge.dataset.desc;
    const icon = badge.dataset.icon;
    const category = badge.dataset.category || 'Medals';
    const isUnlocked = badge.dataset.unlocked === 'true';
    const achieved = badge.dataset.achieved;

    $('medal-tooltip-category').textContent = category;

    const imgEl = $('medal-tooltip-img');
    imgEl.src = `https://assets.ppy.sh/medals/client/${icon}@2x.png`;
    imgEl.onerror = () => {
      imgEl.src = `https://assets.ppy.sh/medals/web/${icon}.png`;
    };
    imgEl.className = 'medal-tooltip__img ' + (isUnlocked ? 'medal-tooltip__img--unlocked' : 'medal-tooltip__img--locked');

    $('medal-tooltip-name').textContent = name;
    $('medal-tooltip-desc').textContent = desc;

    const statusEl = $('medal-tooltip-status');
    const dateEl = $('medal-tooltip-date');
    if (isUnlocked) {
      statusEl.textContent = 'Unlocked';
      statusEl.className = 'medal-tooltip__status medal-tooltip__status--unlocked';
      if (achieved) {
        dateEl.textContent = `Achieved ${achieved}`;
        dateEl.style.display = 'block';
      } else {
        dateEl.textContent = '';
        dateEl.style.display = 'none';
      }
    } else {
      statusEl.textContent = 'Locked';
      statusEl.className = 'medal-tooltip__status medal-tooltip__status--locked';
      dateEl.textContent = '';
      dateEl.style.display = 'none';
    }

    medalTooltipEl.classList.add('visible');

    const rect = badge.getBoundingClientRect();
    const tipW = medalTooltipEl.offsetWidth || 240;
    const tipH = medalTooltipEl.offsetHeight || 290;
    let left = rect.left + rect.width / 2 - tipW / 2;
    left = Math.max(12, Math.min(window.innerWidth - tipW - 12, left));

    let isAbove = true;
    let top = rect.top - tipH - 12;
    if (top < 12) {
      top = rect.bottom + 12;
      isAbove = false;
    }
    medalTooltipEl.classList.toggle('arrow-down', isAbove);
    medalTooltipEl.classList.toggle('arrow-up', !isAbove);

    const arrowEl = medalTooltipEl.querySelector('.medal-tooltip__arrow');
    if (arrowEl) {
      const badgeCenterX = rect.left + rect.width / 2;
      const arrowX = Math.max(18, Math.min(tipW - 18, badgeCenterX - left));
      arrowEl.style.left = `${arrowX}px`;
    }

    medalTooltipEl.style.left = `${left + window.scrollX}px`;
    medalTooltipEl.style.top = `${top + window.scrollY}px`;
  };

  const hideTip = () => {
    if (medalTooltipEl) medalTooltipEl.classList.remove('visible');
  };

  container.addEventListener('mouseover', (e) => {
    const badge = e.target.closest('.badge-achievement');
    if (badge) showTip(badge);
  });
  container.addEventListener('mouseout', (e) => {
    const badge = e.target.closest('.badge-achievement');
    if (badge) hideTip();
  });
  container.addEventListener('focusin', (e) => {
    const badge = e.target.closest('.badge-achievement');
    if (badge) showTip(badge);
  });
  container.addEventListener('focusout', (e) => {
    const badge = e.target.closest('.badge-achievement');
    if (badge) hideTip();
  });
}

async function renderMedals() {
  const container = $('medals-content');
  if (!container) return;

  const catalog = await loadMedalsCatalog();
  if (!catalog || !catalog.length) {
    container.innerHTML = '<p class="empty-inline">Unable to load medals catalog.</p>';
    return;
  }

  const userMedals = state.profile?.medals || [];
  const userMedalsMap = new Map();
  for (const m of userMedals) {
    userMedalsMap.set(m.id, m);
  }

  if ($('medals-count-badge')) {
    $('medals-count-badge').textContent = `${num(userMedals.length)} / ${num(catalog.length)}`;
  }

  const recentUserAchievements = [];
  const sortedUserMedals = [...userMedals].sort((a, b) => (b.achieved_at || 0) - (a.achieved_at || 0));
  for (const um of sortedUserMedals) {
    const fullMedal = catalog.find(c => c.id === um.id) || um;
    recentUserAchievements.push({
      ...fullMedal,
      achieved_at: um.achieved_at,
    });
    if (recentUserAchievements.length >= 8) break;
  }

  const sections = [
    { id: 'beatmap-challenge-packs', title: 'Beatmap Challenge Packs', isHush: false, filter: m => m.category === 'Beatmap Packs' && m.ordering === 5 },
    { id: 'beatmap-packs', title: 'Beatmap Packs', isHush: false, filter: m => m.category === 'Beatmap Packs' && (m.ordering === 6 || m.ordering === 8) },
    { id: 'beatmap-spotlights', title: 'Beatmap Spotlights', isHush: false, filter: m => m.category === 'Beatmap Packs' && m.ordering === 7 },
    { id: 'hush-hush', title: 'Hush-Hush', isHush: true, filter: m => m.category === 'Hush-Hush' && m.ordering === 2 },
    { id: 'hush-hush-expert', title: 'Hush-Hush (Expert)', isHush: true, filter: m => m.category === 'Hush-Hush' && m.ordering === 3 },
    { id: 'skill-dedication', title: 'Skill & Dedication', isHush: false, filter: m => m.category === 'Skill & Dedication' },
    { id: 'mod-introduction', title: 'Mod Introduction', isHush: false, filter: m => m.category === 'Mod Introduction' },
  ];

  if (T19) {
    const fmt = t => t ? new Date(t * 1000).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' }) : '';
    const badge = (m, unlocked, hush, achieved) => {
      const name = (!unlocked && hush) ? '???' : m.name;
      const desc = (!unlocked && hush) ? '???' : m.description;
      return `<div class="badge-achievement y19-medal ${unlocked ? 'badge-achievement--unlocked' : 'badge-achievement--locked'}" tabindex="0" role="button" data-id="${m.id}" data-name="${esc(name)}" data-desc="${esc(desc)}" data-category="${esc(m.category)}" data-icon="${esc(m.icon_url)}" data-unlocked="${unlocked}" data-achieved="${esc(achieved || '')}" aria-label="${esc(name)}"><img class="badge-achievement__image y19-medal__img" src="https://assets.ppy.sh/medals/client/${esc(m.icon_url)}@2x.png" alt="${esc(name)}" loading="lazy" onerror="this.onerror=null;this.src='https://assets.ppy.sh/medals/web/${esc(m.icon_url)}.png';"></div>`;
    };
    let h19 = '';
    if (recentUserAchievements.length) {
      h19 += `<div class="y19-medal-latest"><h3 class="y19-card__small">Latest</h3><div class="y19-medal-latest__row">${recentUserAchievements.map(m => badge(m, true, false, fmt(m.achieved_at))).join('')}</div></div>`;
    }
    h19 += '<div class="y19-medal-cats">';
    for (const sec of sections) {
      const items = catalog.filter(sec.filter).filter(m => !sec.isHush || userMedalsMap.has(m.id));
      if (!items.length) continue;
      h19 += `<div class="y19-medal-cat"><div class="y19-medal-cat__title">${esc(sec.title)}</div><div class="y19-medal-cat__grid">${items.map(m => { const um = userMedalsMap.get(m.id); return badge(m, !!um, sec.isHush, um ? fmt(um.achieved_at) : ''); }).join('')}</div></div>`;
    }
    container.innerHTML = h19 + '</div>';
    setupMedalTooltips();
    return;
  }

  let html = '';

  if (recentUserAchievements.length > 0) {
    html += `<div class="page-extra__recent-medals-box">`;
    html += `  <h3 class="title title--page-extra-small">Latest</h3>`;
    html += `  <div class="page-extra__recent-medals">`;
    for (const m of recentUserAchievements) {
      const dateStr = m.achieved_at 
        ? new Date(m.achieved_at * 1000).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' }) 
        : '';
      html += `<div class="badge-achievement badge-achievement--dynamic-height badge-achievement--unlocked" tabindex="0" role="button"`;
      html += ` data-id="${m.id}"`;
      html += ` data-name="${esc(m.name)}"`;
      html += ` data-desc="${esc(m.description)}"`;
      html += ` data-category="${esc(m.category)}"`;
      html += ` data-icon="${esc(m.icon_url)}"`;
      html += ` data-unlocked="true"`;
      html += ` data-achieved="${esc(dateStr)}"`;
      html += ` aria-label="${esc(m.name)}">`;
      html += `  <img class="badge-achievement__image"`;
      html += `       src="https://assets.ppy.sh/medals/client/${esc(m.icon_url)}@2x.png"`;
      html += `       alt="${esc(m.name)}" loading="lazy"`;
      html += `       onerror="this.onerror=null;this.src='https://assets.ppy.sh/medals/web/${esc(m.icon_url)}.png';">`;
      html += `</div>`;
    }
    html += `  </div>`;
    html += `</div>`;
  }

  for (const sec of sections) {
    const items = catalog.filter(sec.filter).filter(m => !sec.isHush || userMedalsMap.has(m.id));
    if (!items.length) continue;

    html += `<div class="medals-group__group" id="medals-sec-${sec.id}">`;
    html += `  <h3 class="medals-group__title">${esc(sec.title)}</h3>`;
    html += `  <div class="medals-grid">`;
    for (const m of items) {
      const userMedal = userMedalsMap.get(m.id);
      const isUnlocked = !!userMedal;
      const dateStr = userMedal && userMedal.achieved_at 
        ? new Date(userMedal.achieved_at * 1000).toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' }) 
        : '';

      const displayName = (!isUnlocked && sec.isHush) ? '???' : m.name;
      const displayDesc = (!isUnlocked && sec.isHush) ? '???' : m.description;
      const stateCls = isUnlocked ? 'badge-achievement--unlocked' : 'badge-achievement--locked';

      html += `<div class="badge-achievement ${stateCls}" tabindex="0" role="button"`;
      html += ` data-id="${m.id}"`;
      html += ` data-name="${esc(displayName)}"`;
      html += ` data-desc="${esc(displayDesc)}"`;
      html += ` data-category="${esc(m.category)}"`;
      html += ` data-icon="${esc(m.icon_url)}"`;
      html += ` data-unlocked="${isUnlocked}"`;
      html += ` data-achieved="${esc(dateStr)}"`;
      html += ` aria-label="${esc(displayName)}">`;
      html += `  <img class="badge-achievement__image"`;
      html += `       src="https://assets.ppy.sh/medals/client/${esc(m.icon_url)}@2x.png"`;
      html += `       alt="${esc(displayName)}" loading="lazy"`;
      html += `       onerror="this.onerror=null;this.src='https://assets.ppy.sh/medals/web/${esc(m.icon_url)}.png';">`;
      html += `</div>`;
    }
    html += `  </div>`;
    html += `</div>`;
  }

  container.innerHTML = html;
  setupMedalTooltips();
}
function scoreLevel(total) {
  const formula=n=>5000/3*(4*n**3-3*n**2-n)+1.25*1.8**(n-60);
  let level=1, lower=0, upper=0;
  if(total>=26931190827){
    level=100+Math.floor((total-26931190827)/99999999999);
    lower=26931190827+(level-100)*99999999999;upper=lower+99999999999;
  }else{
    while(level<100){upper=lower+Math.round(formula(level+1)-formula(level));if(total<upper)break;lower=upper;level++;}
  }
  return {level,progress:Math.max(0,Math.min(100,Math.floor((total-lower)/(upper-lower)*100)))};
}
const countryNames=new Intl.DisplayNames(['en'],{type:'region'});
const countryName=code=>{
  if(!code||typeof code!=='string')return '';
  const clean=code.trim().toUpperCase();
  try{return countryNames.of(clean)||clean;}catch(_){return clean;}
};
function countryFlagHex(code){
  if(!code||typeof code!=='string')return '';
  const clean=code.trim().toUpperCase();
  if(clean.length!==2)return '';
  return [...clean].map(c=>(127397+c.charCodeAt(0)).toString(16)).join('-');
}
function countryFlagBackground(code){
  const hex=countryFlagHex(code);
  if(!hex)return 'none';
  return `url('https://osu.ppy.sh/assets/images/flags/${hex}.svg'),url('https://cdn.jsdelivr.net/gh/twitter/twemoji@14.0.2/assets/svg/${hex}.svg')`;
}
function parseBBCode(raw) {
  if (!raw) return '';
  if (window.Bbcode) return window.Bbcode.render(raw);
  let s = esc(raw);
  s = s.replace(/\[b\]([\s\S]*?)\[\/b\]/gi, '<strong>$1</strong>');
  s = s.replace(/\[i\]([\s\S]*?)\[\/i\]/gi, '<em>$1</em>');
  s = s.replace(/\[u\]([\s\S]*?)\[\/u\]/gi, '<u>$1</u>');
  s = s.replace(/\[(?:s|strike)\]([\s\S]*?)\[\/(?:s|strike)\]/gi, '<del>$1</del>');
  s = s.replace(/\[heading\]([\s\S]*?)\[\/heading\]/gi, '<h3 class="bbcode-heading">$1</h3>');
  s = s.replace(/\[(?:centre|center)\]([\s\S]*?)\[\/(?:centre|center)\]/gi, '<div class="bbcode-centre">$1</div>');
  s = s.replace(/\[color=([#\w]+)\]([\s\S]*?)\[\/color\]/gi, (m, c, t) => /^[#a-zA-Z0-9]+$/.test(c) ? `<span style="color:${c}">${t}</span>` : t);
  s = s.replace(/\[size=(\d+)\]([\s\S]*?)\[\/size\]/gi, (m, sz, t) => `<span style="font-size:${Math.min(Math.max(Number(sz)||100,50),300)}%">${t}</span>`);
  s = s.replace(/\[url=(https?:\/\/[^\s"'>]+)\]([\s\S]*?)\[\/url\]/gi, '<a href="$1" target="_blank" rel="noreferrer" class="bbcode-link">$2</a>');
  s = s.replace(/\[url\](https?:\/\/[^\s"'>]+)\[\/url\]/gi, '<a href="$1" target="_blank" rel="noreferrer" class="bbcode-link">$1</a>');
  s = s.replace(/\[img\](https?:\/\/[^\s"'>]+)\[\/img\]/gi, '<img src="$1" alt="image" class="bbcode-image" loading="lazy">');
  s = s.replace(/\[box=([^\]]+)\]([\s\S]*?)\[\/box\]/gi, '<details class="bbcode-box"><summary class="bbcode-box__header">$1</summary><div class="bbcode-box__content">$2</div></details>');
  s = s.replace(/\[box\]([\s\S]*?)\[\/box\]/gi, '<details class="bbcode-box"><summary class="bbcode-box__header">Show details</summary><div class="bbcode-box__content">$1</div></details>');
  s = s.replace(/\[spoiler\]([\s\S]*?)\[\/spoiler\]/gi, '<span class="bbcode-spoiler" title="Spoiler">$1</span>');
  s = s.replace(/\[quote=([^\]]+)\]([\s\S]*?)\[\/quote\]/gi, '<blockquote class="bbcode-quote"><div class="bbcode-quote__author">$1 wrote:</div><div class="bbcode-quote__content">$2</div></blockquote>');
  s = s.replace(/\[quote\]([\s\S]*?)\[\/quote\]/gi, '<blockquote class="bbcode-quote"><div class="bbcode-quote__content">$1</div></blockquote>');
  s = s.replace(/\[list\]([\s\S]*?)\[\/list\]/gi, (m, items) => {
    const listItems = items.split(/\[\*\]/).filter(x => x.trim().length > 0).map(x => `<li>${x.trim()}</li>`).join('');
    return `<ul class="bbcode-list">${listItems}</ul>`;
  });
  s = s.replace(/\n/g, '<br>');
  s = s.replace(/<br>\s*(<\/?(?:div|details|summary|blockquote|h3|ul|li))/gi, '$1');
  s = s.replace(/(<\/(?:div|details|summary|blockquote|h3|ul|li)>)\s*<br>/gi, '$1');
  return s;
}

function renderProfile() {
  if (T16 && window.renderProfile16) { window.renderProfile16(); return; }
  const p = state.profile;
  document.title = `${p.name} · player info · Local osu!`;
  $('username').textContent = p.name;
  applyCover(p);
  const details=p.details||{};
  for(const id of fixedSectionOrder()||details.section_order||defaultSectionOrder){
    const section=$(id),link=document.querySelector(`.section-tabs a[href="#${id}"]`);
    if(section&&link){document.querySelector('.user-profile-pages').append(section);document.querySelector('.section-tabs').append(link);}
  }
  const countryCode=(details.country||'').trim().toUpperCase();
  const countryEl=$('profile-country');
  if(countryCode&&countryCode.length===2){
    const cName=countryName(countryCode)||countryCode;
    countryEl.href=`https://osu.ppy.sh/rankings/osu/performance?country=${encodeURIComponent(countryCode)}`;
    countryEl.title=cName;
    countryEl.setAttribute('aria-label',cName);
    countryEl.innerHTML=`<span class="flag-country flag-country--medium" style="background-image:${countryFlagBackground(countryCode)}" aria-hidden="true"></span><span class="profile-info__flag-text">${esc(cName)}</span>`;
    countryEl.removeAttribute('hidden');
    countryEl.style.display='';
  }else{
    countryEl.removeAttribute('href');
    countryEl.removeAttribute('title');
    countryEl.replaceChildren();
    countryEl.style.display='none';
  }
  const linksRow = $('profile-links-row');
  if (linksRow) {
    linksRow.replaceChildren();

    if (NEXT && !T19) {
      const cc = (details.country || '').trim().toUpperCase();
      if (cc.length === 2) {
        const fromItem = document.createElement('span');
        fromItem.className = 'profile-links__item';
        fromItem.innerHTML = `From <span class="profile-links__value">${esc(countryName(cc) || cc)}</span>`;
        linksRow.appendChild(fromItem);
      }
    }

    const joinDateVal = details.join_date ? details.join_date + '-01T12:00:00' : p.first_play;
    let joinDateText = '';
    if (joinDateVal) {
      const d = typeof joinDateVal === 'number' ? new Date(joinDateVal * 1000) : new Date(joinDateVal);
      if (!isNaN(d.getTime())) {
        joinDateText = d.toLocaleDateString('en-US', { month: 'long', year: 'numeric' });
      }
    }
    if (!joinDateText) {
      joinDateText = new Date().toLocaleDateString('en-US', { month: 'long', year: 'numeric' });
    }
    const joinItem = document.createElement('span');
    joinItem.className = 'profile-links__item';
    joinItem.innerHTML = `Joined <span class="profile-links__value">${esc(joinDateText)}</span>`;
    linksRow.appendChild(joinItem);

    const statusItem = document.createElement('span');
    statusItem.className = 'profile-links__item';
    if (p.active) {
      statusItem.innerHTML = `<span class="profile-links__value">Currently online</span>`;
    } else if (p.last_play) {
      statusItem.innerHTML = `Last seen <span class="profile-links__value">${relative(p.last_play)}</span>`;
    } else {
      statusItem.innerHTML = `Last seen <span class="profile-links__value">recently</span>`;
    }
    linksRow.appendChild(statusItem);

    if (details.devices && details.devices.length) {
      const devItem = document.createElement('span');
      devItem.className = 'profile-links__item';
      devItem.innerHTML = `Plays with <span class="profile-links__value">${esc(details.devices.join(', '))}</span>`;
      linksRow.appendChild(devItem);
    }

    if (details.location && details.location.trim()) {
      const locItem = document.createElement('span');
      locItem.className = 'profile-links__item';
      locItem.innerHTML = `<span class="profile-links__icon"><i class="fas fa-map-marker-alt" aria-hidden="true"></i></span><span class="profile-links__value">${esc(details.location.trim())}</span>`;
      locItem.classList.add('profile-links__item--info');
      locItem.dataset.y19loc = '1';
      linksRow.appendChild(locItem);
    }

    const d19 = details;
    const extras = [];
    const clean = v => (v || '').trim();
    const safeUrl = u => { u = clean(u); if (!u) return ''; if (!/^[a-z][a-z0-9+.-]*:/i.test(u)) u = 'https://' + u; return /^https?:\/\//i.test(u) ? u : ''; };
    if (clean(d19.interests)) extras.push(['i', 'far fa-heart', esc(clean(d19.interests)), '']);
    if (clean(d19.occupation)) extras.push(['i', 'fas fa-suitcase', esc(clean(d19.occupation)), '']);
    const tw = clean(d19.twitter).replace(/^@/, '').replace(/^https?:\/\/(www\.)?(twitter|x)\.com\//i, '').replace(/[/?#].*$/, '');
    if (tw && /^\w{1,50}$/.test(tw)) extras.push(['s', 'fab fa-twitter', '@' + esc(tw), 'https://twitter.com/' + encodeURIComponent(tw)]);
    const web = safeUrl(d19.website);
    if (web) extras.push(['s', 'fas fa-link', esc(clean(d19.website)), web]);
    if (clean(d19.discord)) extras.push(['s', 'fab fa-discord', esc(clean(d19.discord)), '']);
    extras.forEach(([group, icon, text, href]) => {
      const it = document.createElement('span');
      it.className = 'profile-links__item profile-links__item--' + (group === 's' ? 'social' : 'info');
      it.dataset.y19group = group;
      const val = href ? `<a class="profile-links__value y19-link" href="${esc(href)}" target="_blank" rel="noreferrer noopener">${text}</a>` : `<span class="profile-links__value">${text}</span>`;
      it.innerHTML = `<span class="profile-links__icon"><i class="${icon}" aria-hidden="true"></i></span>${val}`;
      linksRow.appendChild(it);
    });

    if (details.post_count != null && details.post_count > 0) {
      const postItem = document.createElement('span');
      postItem.className = 'profile-links__item';
      postItem.innerHTML = `Contributed <span class="profile-links__value${T19?' y19-link':''}">${num(details.post_count)} forum ${details.post_count === 1 ? 'post' : 'posts'}</span>`;
      linksRow.appendChild(postItem);
    }

    if (details.comments_count != null && details.comments_count > 0) {
      const commItem = document.createElement('span');
      commItem.className = 'profile-links__item';
      commItem.innerHTML = `Posted <span class="profile-links__value${T19?' y19-link':''}">${num(details.comments_count)} ${details.comments_count === 1 ? 'comment' : 'comments'}</span>`;
      linksRow.appendChild(commItem);
    }
    if (T19) {
      const mk = () => { const br = document.createElement('span'); br.className = 'y19-br'; return br; };
      const loc = linksRow.querySelector('[data-y19loc]');
      const info = [...linksRow.querySelectorAll('[data-y19group="i"]')];
      const soc = [...linksRow.querySelectorAll('[data-y19group="s"]')];
      if (loc || info.length) { linksRow.appendChild(mk()); if (loc) linksRow.appendChild(loc); info.forEach(n => linksRow.appendChild(n)); }
      if (soc.length) { const b2 = mk(); if (loc || info.length) b2.classList.add('y19-br--tight'); linksRow.appendChild(b2); soc.forEach(n => linksRow.appendChild(n)); }
    } else if (!NEXT && !T16) {
      const items = [...linksRow.children];
      const isInfo = n => n.dataset.y19loc || n.dataset.y19group === 'i';
      const isSocial = n => n.dataset.y19group === 's';
      const rows = [
        ['stats', items.filter(n => !isInfo(n) && !isSocial(n))],
        ['info', items.filter(isInfo)],
        ['social', items.filter(isSocial)],
      ];
      linksRow.textContent = '';
      linksRow.classList.add('profile-links__row--grouped');
      rows.forEach(([kind, nodes]) => {
        if (!nodes.length) return;
        const g = document.createElement('div');
        g.className = 'profile-links__group profile-links__group--' + kind;
        nodes.forEach(n => g.appendChild(n));
        linksRow.appendChild(g);
      });
    }
  }
  if(details.about){
    $('about-content').innerHTML=parseBBCode(details.about);
    $('about-content').classList.remove('subdued');
  }else{
    $('about-content').textContent="This player hasn't written anything about themselves yet.";
    $('about-content').classList.add('subdued');
  }
  $('avatar').src = `/site/avatar?name=${encodeURIComponent(p.name)}&v=${state.avatarVersion||AVATAR_BOOT}`;
  $('edit-profile').hidden=state.session.user!==p.name;
  $('edit-profile-details').hidden=state.session.user!==p.name;
  $('edit-about').hidden=state.session.user!==p.name;
  const level=scoreLevel(p.total_score);
  $('level-number').textContent=level.level;
  $('level-badge').title=`Level ${level.level} · calculated from saved total score`;
  $('level-fill').style.width=`${level.progress}%`;
  $('level-percent').textContent=`${level.progress}%`;
  $('level-progress').setAttribute('aria-valuenow',level.progress);
  $('avatar').alt = `${p.name}'s avatar`;
  $('presence').textContent = p.active ? 'Active' : '';
  $('presence').classList.toggle('active', p.active);
  if(p.global_rank){
    const highest=getHighestRank(p);
    const highestHtml=highest?`<div class="rank-highest-popup">Highest rank: #${num(highest.rank)} on ${formatHighestDate(highest.updated_at)}</div>`:'';
    const tier=NEXT?null:globalRankTier(p.global_rank);
    const rankHtml=tier?`<span class="rank-value rank-value--${tier}" style="--colour:var(--level-tier-${tier})">#${num(p.global_rank)}</span>`:`#${num(p.global_rank)}`;
    $('global-rank').title=p.pp_system&&p.pp_system!=='live'&&state.ppfollow?'Rank this pp would have had on the ladder when the selected pp system ended (osu!track data)':p.rank_era&&p.rank_era!=='latest'?`Rank this pp would have had on the ${p.rank_era_date||p.rank_era} ladder (osu!track data)`:'';
  $('global-rank').innerHTML=`${rankHtml}${(NEXT&&!T19)?'':highestHtml}`;
    $('global-rank').removeAttribute('title');
  }else{
    $('global-rank').textContent='—';
    $('global-rank').title='osu!daily rank unavailable. Check the configured API key or try again later.';
  }
  if($('country-rank')){
    if(p.country_rank){
      $('country-rank').textContent=`#${num(p.country_rank)}`;
      $('country-rank').removeAttribute('title');
    }else{
      $('country-rank').textContent='—';
      $('country-rank').title='Country rank unavailable. Requires valid osu! API OAuth credentials, configured country, and ranked plays.';
    }
  }
  $('pp').textContent = num(p.pp);
  { const ex = Number(p.pp_exact); const t = ex > 0 ? ex.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 4 }) + 'pp' : ''; $('pp').title = t; if ($('pp').parentElement) $('pp').parentElement.title = t; }
  if (!NEXT) { $('country-rank').textContent = p.country_rank ? `#${num(p.country_rank)}` : '-'; }
  if (NEXT) {
    const cr = $('country-rank'), cc = ((p.details || {}).country || '').trim().toUpperCase();
    cr.textContent = (p.country_rank && cc) ? (T19 ? `#${num(p.country_rank)}` : `${countryName(cc) || cc} #${num(p.country_rank)}`) : (T19 ? '-' : (cc ? `${countryName(cc) || cc} -` : '-'));
  }
  const playTime = s => { s = Math.floor(s || 0); const d = Math.floor(s / 86400), h = Math.floor(s % 86400 / 3600), m = Math.floor(s % 3600 / 60); return d ? `${d}d ${h}h ${m}m` : (h ? `${h}h ${m}m` : `${m}m`); };
  const stats = [['Ranked Score',num(p.ranked_score)],['Hit Accuracy',`${num(p.acc,2)}%`],['Play Count',num(p.playcount)],['Play Time',p.play_time_secs > 0 ? playTime(p.play_time_secs) : '—'],['Total Score',num(p.total_score)],['Total Hits',num(p.total_hits)],['Hits per Play',num(p.playcount ? Math.floor(p.total_hits/p.playcount) : 0)],['Maximum Combo',`${num(p.max_combo)}x`],['Replays Watched by Others',p.official_stats?num(p.replays_watched):'—']];
  $('statistics').innerHTML = stats.map(([key,value])=>`<dl class="profile-stats__entry"><dt class="profile-stats__key">${key}</dt><dd class="profile-stats__value">${value}</dd></dl>`).join('');
  const gradeOrder = ['XH', 'X', 'SH', 'S', 'A'];
  const gradeLabels = { XH: 'SSH', X: 'SS', SH: 'SH', S: 'S', A: 'A' };
  $('grade-counts').innerHTML = gradeOrder.map(grade => {
    const label = gradeLabels[grade];
    const count = (p.grades && p.grades[grade] != null) ? p.grades[grade] : ((p.grades && p.grades[label] != null) ? p.grades[label] : 0);
    return `<div class="profile-rank-count__item"><div class="profile-rank-count__rank"><div class="score-rank score-rank--${grade} score-rank--rank-${label.toLowerCase()}" role="img" aria-label="${label}"></div></div><span>${num(count)}</span></div>`;
  }).join('');
  const modeLabel = state.mode === 'rx' ? 'relax' : state.mode === 'ap' ? 'autopilot' : 'osu!';
  const activityRow = NEXT
    ? (s => `<div class="activity-row" data-score="${esc(s.id)}"><span class="score-rank score-rank--tiny score-rank--${esc(s.grade)}" aria-label="Grade ${esc(s.grade)}"></span><div><a class="nx-activity__user" href="/users/${encodeURIComponent(p.name)}?mode=${state.mode}">${esc(p.name)}</a> achieved a <b>${num(s.pp)}pp</b> play on <a ${Number(s.beatmap.beatmap_id)>0?`href="/beatmaps/${Number(s.beatmap.beatmap_id)}"`:''}>${esc(s.beatmap.title||'an unknown beatmap')} [${esc(s.beatmap.version||'?')}]</a> (${modeLabel})</div><time>${relative(s.time)}</time></div>`)
    : null;
  const modernRow = s => {
    const bm = s.beatmap || {}; const bid = Number(bm.beatmap_id);
    const title = `${bm.artist && bm.artist !== 'unknown artist' ? esc(bm.artist) + ' - ' : ''}${esc(bm.title || 'an unknown beatmap')} [${esc(bm.version || '?')}]`;
    return `<div class="activity-row" data-score="${esc(s.id)}"><span class="score-rank score-rank--tiny score-rank--${esc(s.grade)}" aria-label="Grade ${esc(s.grade)}"></span><div class="activity-row__text"><strong>${esc(p.name)}</strong> played <a ${bid>0?`href="/beatmaps/${bid}"`:''}>${title}</a> with <strong>${num(s.pp)}pp</strong> (${modeLabel})</div><time>${relative(s.time)}</time></div>`;
  };
  state.renderAct = () => {
    const actLimit = NEXT ? 5 : state.actLimit;
    $('activity').innerHTML = p.recent.length ? p.recent.slice(0,actLimit).map(s=>activityRow?activityRow(s):modernRow(s)).join('') : '<p class="empty-inline">No recent activity.</p>';
    if ($('more-activity')) $('more-activity').hidden = NEXT || p.recent.length <= state.actLimit;
  };
  state.renderAct();
  const medalsCount = (p.medals && p.medals.length) || 0;
  if ($('medals-count')) $('medals-count').textContent = num(medalsCount);
  if ($('play-time')) $('play-time').textContent = p.play_time_secs > 0 ? playTime(p.play_time_secs) : '—';
  if ($('medals-count-badge')) $('medals-count-badge').textContent = num(medalsCount);
  renderMedals();
  renderPerformanceGraph();
  requestAnimationFrame(updateSection);
  renderScores(); renderHistory(); renderMostPlayed();
  updateModeSelector();
  $('welcome').hidden = true;
  $('profile-content').hidden = false;
}
async function loadProfile(name, navigate = false, quiet = false) {
  if (!name) { $('welcome').hidden = false; $('profile-content').hidden = true; return; }
  const request = ++state.request;
  if (!quiet) showMessage('Loading profile…');
  try {
    const profile = await api(`/site/profile?name=${encodeURIComponent(name)}&mode=${state.mode}${state.era!=='latest'?`&era=${state.era}`:''}${state.ppsys!=='live'?`&ppsys=${state.ppsys}${state.ppfollow?'':'&ppfollow=0'}`:''}`);
    if (request !== state.request) return;
    if ((NEXT || T16) && Array.isArray(profile.top)) profile.top = profile.top.slice(0, 100);
    state.profile = profile; state.name = name;
    if (navigate) history.pushState({},'',`/users/${encodeURIComponent(name)}?mode=${state.mode}`);
    renderProfile(); showMessage('');
    if (state.showPpSystemNote) state.showPpSystemNote(profile);
  } catch (error) {
    if (request !== state.request) return;
    if (!quiet) { $('profile-content').hidden = true; showMessage(error.message); }
  }
}
async function boot() {
  try {
    state.session = await api('/site/session');
    renderSession();
    if (location.hash === '#login') { history.replaceState({}, '', location.pathname + location.search); openLogin(); }
    if (location.hash === '#settings' && state.session.user) { history.replaceState({}, '', location.pathname + location.search); setTimeout(() => openSettings(), 300); }
    const query = new URLSearchParams(location.search);
    state.mode = ['osu','taiko','fruits','mania','vn','rx','ap'].includes(query.get('mode')) ? query.get('mode') : 'osu';
    if (state.mode === 'vn') state.mode = 'osu';
    updateModeSelector();
    let name = /\/(?:users|u)\/([^/]+)/.exec(location.pathname)?.[1];
    if (name) name = decodeURIComponent(name);
    if (name === '2' && !state.session.profiles.includes('2')) name = state.session.active;
    await loadProfile(name || state.session.user || state.session.active || state.session.default);
  } catch (error) { showMessage(`Cannot load the local server: ${error.message}`); }
}
function closeAccount(){ $('account-panel').hidden=true;$('account').setAttribute('aria-expanded','false'); }
$('account').addEventListener('click',()=>{
  if(!state.session?.user){openLogin();return;}
  $('account-panel').hidden=!$('account-panel').hidden;
  $('account').setAttribute('aria-expanded',String(!$('account-panel').hidden));
});
$('account-switch').addEventListener('click',openLogin);
$('account-settings').addEventListener('click',()=>{closeAccount();openSettings();});
$('account-logout').addEventListener('click',()=>{closeAccount();$('logout').click();});
document.addEventListener('click',event=>{if(!event.target.closest('#account-panel,#account'))closeAccount();});
document.addEventListener('keydown',event=>{if(event.key==='Escape'&&!$('account-panel').hidden){closeAccount();$('account').focus();}});
$('mobile-account').addEventListener('click',openLogin);
$('mobile-menu-toggle').addEventListener('click',()=>{const hidden=!$('mobile-menu').hidden;$('mobile-menu').hidden=hidden;$('mobile-menu-toggle').setAttribute('aria-expanded',String(!hidden));});
$('first-login').addEventListener('click',openLogin);
$('close-login').addEventListener('click',()=>$('login-dialog').close());
$('login-dialog').addEventListener('click',event=>{if(event.target===$('login-dialog')){const r=$('login-dialog').getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)$('login-dialog').close();}});
$('login-form').addEventListener('submit',async event=>{
  event.preventDefault(); $('login-error').hidden=true; $('login-submit').disabled=true;
  try {
    if(!state.session) state.session=await api('/site/session');
    const result=await api('/site/login',{username:$('login-username').value});
    state.session=await api('/site/session'); renderSession();
    state.topLimit=5;state.firstLimit=5;state.recentLimit=5;state.actLimit=5;
    await loadProfile(result.name,true);$('login-dialog').close();
  } catch(error){$('login-error').textContent=error.message;$('login-error').hidden=false;}
  finally{$('login-submit').disabled=false;}
});
$('logout').addEventListener('click',async()=>{
  try{await api('/site/logout',{});state.session=await api('/site/session');renderSession();renderProfile();$('login-dialog').close();}
  catch(error){$('login-error').textContent=error.message;$('login-error').hidden=false;}
});
function updateModeSelector(){
  const currentRuleset = ['osu', 'vn', 'rx', 'ap'].includes(state.mode) ? 'osu' : (state.mode || 'osu');

  document.querySelectorAll('#game-mode-nav .game-mode-link').forEach(btn => {
    const btnMode = btn.dataset.mode;
    const isActive = btnMode === currentRuleset;
    btn.classList.toggle('game-mode-link--active', isActive);
    btn.setAttribute('aria-pressed', String(isActive));

    if (btnMode === 'osu') {
      let badge = btn.querySelector('.game-mode-link__badge');
      if (state.mode === 'rx') {
        if (!badge) {
          badge = document.createElement('span');
          badge.className = 'game-mode-link__badge';
          btn.appendChild(badge);
        }
        badge.textContent = 'rx';
        btn.title = 'osu! (Relax)';
        btn.setAttribute('aria-label', 'osu! (Relax)');
      } else if (state.mode === 'ap') {
        if (!badge) {
          badge = document.createElement('span');
          badge.className = 'game-mode-link__badge';
          btn.appendChild(badge);
        }
        badge.textContent = 'ap';
        btn.title = 'osu! (Autopilot)';
        btn.setAttribute('aria-label', 'osu! (Autopilot)');
      } else {
        if (badge) badge.remove();
        btn.title = 'osu!';
        btn.setAttribute('aria-label', 'osu!');
      }
    }
  });
}
document.querySelectorAll('#game-mode-nav .game-mode-link').forEach(btn => {
  btn.addEventListener('click', async () => {
    const clickedMode = btn.dataset.mode;
    const currentRuleset = ['osu', 'vn', 'rx', 'ap'].includes(state.mode) ? 'osu' : state.mode;

    let targetMode = clickedMode;
    if (clickedMode === 'osu') {
      if (currentRuleset === 'osu') {
        if (state.mode === 'osu' || state.mode === 'vn') {
          targetMode = 'rx';
        } else if (state.mode === 'rx') {
          targetMode = 'ap';
        } else {
          targetMode = 'osu';
        }
      } else {
        targetMode = 'osu';
      }
    } else {
      if (state.mode === clickedMode) return;
      targetMode = clickedMode;
    }

    state.mode = targetMode;
    updateModeSelector();
    state.topLimit = 5; state.firstLimit = 5; state.recentLimit = 5; state.mostLimit = 5;
    await loadProfile(state.name, true);
  });
});
$('more-top').addEventListener('click',()=>{state.topLimit+=10;renderScores();});
$('more-firsts').addEventListener('click',()=>{state.firstLimit+=10;renderScores();});
$('more-activity')&&$('more-activity').addEventListener('click',()=>{state.actLimit+=10;if(state.renderAct)state.renderAct();});
$('more-recent').addEventListener('click',()=>{state.recentLimit+=10;renderScores();});
const defaultSectionOrder=['me','top-ranks','historical','beatmaps','medals','recent-activity'];
let sectionFrame=0;
function updateSection(){
  sectionFrame=0;
  const sectionLinks=[...document.querySelectorAll('.section-tabs a')];
  const offset=matchMedia('(max-width:899px)').matches?105:65;
  let current=NEXT?null:sectionLinks[0];
  for(const link of sectionLinks){const target=document.querySelector(link.hash);if(target&&target.getBoundingClientRect().top<=offset)current=link;}
  for(const link of sectionLinks){link.classList.toggle('selected',link===current);if(link===current)link.setAttribute('aria-current','location');else link.removeAttribute('aria-current');}
}
window.addEventListener('scroll',()=>{if(!sectionFrame)sectionFrame=requestAnimationFrame(updateSection);},{passive:true});
window.addEventListener('resize',updateSection);
window.addEventListener('popstate',boot);
setInterval(async()=>{
  if(document.hidden||$('login-dialog').open||document.querySelector('.score-expanded:not([hidden])')||$('settings-dialog').open||!state.name)return;
  try{state.session=await api('/site/session');renderSession();await loadProfile(state.name,false,true);}catch{}
},30000);
boot();

const fallbackAvatar='/site/static/vendor/avatar-guest@2x.01495bc4.png';
$('account-avatar').addEventListener('error',()=>{const el=$('account-avatar');if(el.src.endsWith(fallbackAvatar))return;if(!el.dataset.retried&&el.src.includes('/site/avatar')){el.dataset.retried='1';el.src=el.src.replace(/([&?])v=[^&]*/,'$1v='+Date.now());return;}el.src=fallbackAvatar;});
$('avatar').addEventListener('error',()=>{const el=$('avatar');if(el.src.endsWith(fallbackAvatar))return;if(!el.dataset.retried&&el.src.includes('/site/avatar')){el.dataset.retried='1';el.src=el.src.replace(/([&?])v=[^&]*/,'$1v='+Date.now());return;}el.src=fallbackAvatar;});
$('settings-avatar').addEventListener('error',()=>{if(!$('settings-avatar').src.endsWith(fallbackAvatar))$('settings-avatar').src=fallbackAvatar;});
$('more-most').addEventListener('click',()=>{state.mostLimit=(state.mostLimit||5)+10;renderMostPlayed();});
$('profile-content').addEventListener('click',event=>{
  const button=event.target.closest('[data-score-detail]');if(!button)return;
  const detail=$('detail-'+button.dataset.scoreDetail);detail.hidden=!detail.hidden;button.setAttribute('aria-expanded',String(!detail.hidden));
});
function updateSettingsCountryFlag(){
  const flagEl=$('settings-country-flag');
  if(!flagEl)return;
  const val=($('settings-country').value||'').trim().toUpperCase();
  if(val&&val.length===2){
    flagEl.style.backgroundImage=countryFlagBackground(val);
    flagEl.style.display='block';
  }else{
    flagEl.style.display='none';
  }
}
function setCoverPreview(src){
  const el=$('settings-cover');
  el.style.backgroundImage=src?`url("${src}")`:'';
  el.classList.toggle('cover-editor__preview--empty',!src);
}
function applyCover(p){
  const on=!!(p&&p.has_cover);
  document.body.classList.toggle('has-cover',on);
  if(on)document.body.style.setProperty('--user-cover',`url("/site/cover?name=${encodeURIComponent(p.name)}&v=${state.avatarVersion||AVATAR_BOOT}")`);
  else document.body.style.removeProperty('--user-cover');
}
async function openSettings(){
  if(!state.session.user){openLogin();return;}
  $('login-dialog').close();
  state.avatarUpload=null;state.resetAvatar=false;state.coverUpload=null;state.resetCover=false;
  $('avatar-file').value='';$('cover-file').value='';setCoverPreview(state.profile&&state.profile.has_cover&&state.profile.name===state.session.user?`/site/cover?name=${encodeURIComponent(state.profile.name)}&v=${Date.now()}`:'');$('settings-error').hidden=true;
  $('import-scores-query').value='';showScoresStatus('','');
  $('settings-username').value=state.session.user;
  $('settings-avatar').src=`/site/avatar?name=${encodeURIComponent(state.session.user)}&v=${Date.now()}`;
  $('settings-dialog').showModal();
  $('settings-submit').disabled=true;
  try{
    const profile=await api(`/site/profile?name=${encodeURIComponent(state.session.user)}&mode=${state.mode}`);
    const details=profile.details||{};
    $('settings-country').value=details.country||'';
    updateSettingsCountryFlag();
    $('settings-location').value=details.location||'';
    ['interests','occupation','twitter','website','discord'].forEach(k=>{$('settings-'+k).value=details[k]||'';});
    $('settings-join').value=details.join_date||'';
    $('settings-about').value=details.about||'';
    state.sectionOrder=[...(details.section_order||defaultSectionOrder)];renderSectionOrder();
    document.querySelectorAll('input[name=device]').forEach(input=>input.checked=(details.devices||[]).includes(input.value));
    $('settings-submit').disabled=false;
  }catch(error){$('settings-error').textContent=error.message;$('settings-error').hidden=false;}
}
function showScoresStatus(msg,type){
  const el=$('import-scores-status');
  el.textContent=msg;el.className='import-status '+(type||'');el.hidden=!msg;
}
$('import-scores-btn').addEventListener('click',async()=>{
  const query=$('import-scores-query').value.trim();
  if(!query){showScoresStatus('Enter an official osu! username or user ID.','error');$('import-scores-query').focus();return;}
  const btn=$('import-scores-btn');
  btn.disabled=true;showScoresStatus('Step 1/3: fetching profile from osu.ppy.sh...','loading');
  let profileMsg='';
  try{
    const res=await api('/site/import_official',{query,apply:false,import_avatar:true,import_cover:true,import_bio:true,import_details:true});
    const details=res.details||{};
    if(details.about!==undefined)$('settings-about').value=details.about;
    if(details.location!==undefined)$('settings-location').value=details.location;
    ['interests','occupation','twitter','website','discord'].forEach(k=>{if(details[k]!==undefined)$('settings-'+k).value=details[k];});
    if(details.join_date!==undefined)$('settings-join').value=details.join_date;
    if(details.country!==undefined){$('settings-country').value=details.country;updateSettingsCountryFlag();}
    if(Array.isArray(details.devices)){
      document.querySelectorAll('input[name=device]').forEach(input=>{input.checked=details.devices.includes(input.value);});
    }
    if(res.avatar_data_url){$('settings-avatar').src=res.avatar_data_url;state.avatarUpload=res.avatar_data_url;state.resetAvatar=false;}
    if(res.cover_data_url){state.coverUpload=res.cover_data_url;state.resetCover=false;setCoverPreview(res.cover_data_url);}
    profileMsg=`Profile of ${res.official_username} (#${res.official_id}) loaded into the form below (click Save changes to keep it)`;
  }catch(err){showScoresStatus(err.message,'error');btn.disabled=false;return;}
  showScoresStatus('Step 2/3: downloading scores from osu.ppy.sh (paced, can take up to a minute)...','loading');
  try{
    const res=await api('/site/import_scores',{query,mode:$('import-scores-mode').value,limit:Number($('import-scores-limit').value)});
    const returned=(res.fetched||[]).reduce((n,f)=>n+f.returned,0);
    const detail=(res.fetched||[]).map(f=>`${f.mode}: ${f.returned}`).join(', ');
    if(returned===0){showScoresStatus(`${profileMsg}. But osu.ppy.sh returned no top scores for ${res.official_name} (${detail}). Check the mode, or whether that account has ranked scores.`,'error');return;}
    const parts=[profileMsg,`Imported ${res.imported} score${res.imported===1?'':'s'} (osu! returned ${detail})`];
    const offi=(res.fetched||[]).filter(f=>f.official_pp).map(f=>`${f.mode} ${num(f.official_pp)}pp`);
    if(offi.length)parts.push(`pp matched to your official total (${offi.join(', ')})`);
    const fs=(res.fetched||[]).reduce((n,f)=>n+(f.firsts||0),0);
    if(fs)parts.push(`${fs} first place rank${fs===1?'':'s'}`);
    if(res.stats_imported)parts.push(`profile statistics (play count, play time, total hits, scores, grades) copied for ${res.stats_imported} mode${res.stats_imported===1?'':'s'}`);
    if(res.skipped_existing)parts.push(`${res.skipped_existing} already imported`);
    if(res.skipped_unusable)parts.push(`${res.skipped_unusable} skipped (no beatmap data)`);
    const base=parts.join(' · ');
    const modes=$('import-scores-mode').value==='all'?['osu','taiko','fruits','mania']:[$('import-scores-mode').value||'osu'];
    if(!$('import-ranks-toggle').checked){
      showScoresStatus(`${base} · leaderboard rank lookup skipped.`,'success');
      if(state.session&&state.session.user)loadProfile(state.session.user,false,true);
      return;
    }
    let checked=0,ranked=0,rankErr='';
    try{
      for(const m of modes){
        let off=0;
        while(off!==null&&off!==undefined){
          showScoresStatus(`Step 3/3: looking up official leaderboard ranks (${m}, ${off}/100)...`,'loading');
          const r=await api('/site/import_ranks',{query,mode:m,offset:off,count:25});
          checked+=r.checked;ranked+=r.ranked;off=r.next_offset;
        }
      }
    }catch(err){rankErr=` · leaderboard ranks stopped: ${err.message}`;}
    showScoresStatus(`${base} · leaderboard ranks for ${ranked} of ${checked} top plays${rankErr}.`,rankErr?'error':'success');
    if(state.session&&state.session.user)loadProfile(state.session.user,false,true);
  }catch(err){showScoresStatus(`${profileMsg}. Scores failed: ${err.message}`,'error');}
  finally{btn.disabled=false;}
});
$('import-scores-query').addEventListener('keydown',e=>{
  if(e.key==='Enter'){e.preventDefault();$('import-scores-btn').click();}
});
$('edit-profile').addEventListener('click',openSettings);
$('edit-profile-details').addEventListener('click',openSettings);
$('manage-profile').addEventListener('click',openSettings);
$('close-settings').addEventListener('click',()=>$('settings-dialog').close());
$('reset-avatar').addEventListener('click',()=>{state.avatarUpload=null;state.resetAvatar=true;$('avatar-file').value='';$('settings-avatar').src=fallbackAvatar;});
$('reset-cover').addEventListener('click',()=>{state.coverUpload=null;state.resetCover=true;$('cover-file').value='';setCoverPreview('');});
$('cover-file').addEventListener('change',async()=>{
  const file=$('cover-file').files[0];if(!file)return;
  $('settings-error').hidden=true;
  if(file.size>6*1024*1024||!['image/png','image/jpeg','image/gif','image/webp'].includes(file.type)){
    $('settings-error').textContent='Choose a PNG, JPEG, GIF or WebP banner up to 6 MB.';$('settings-error').hidden=false;$('cover-file').value='';return;
  }
  const reader=new FileReader();$('settings-submit').disabled=true;
  reader.onload=()=>{if($('cover-file').files[0]===file){state.coverUpload=reader.result;state.resetCover=false;setCoverPreview(reader.result);}$('settings-submit').disabled=false;};
  reader.onerror=()=>{$('settings-error').textContent='The image could not be read.';$('settings-error').hidden=false;$('settings-submit').disabled=false;};
  reader.readAsDataURL(file);
});
$('avatar-file').addEventListener('change',async()=>{
  const file=$('avatar-file').files[0];if(!file)return;
  $('settings-error').hidden=true;
  if(file.size>2*1024*1024||!['image/png','image/jpeg','image/gif','image/webp'].includes(file.type)){
    $('settings-error').textContent='Choose a PNG, JPEG, GIF or WebP up to 2 MB.';$('settings-error').hidden=false;$('avatar-file').value='';return;
  }
  const reader=new FileReader();$('settings-submit').disabled=true;
  reader.onload=()=>{if($('avatar-file').files[0]===file){state.avatarUpload=reader.result;state.resetAvatar=false;$('settings-avatar').src=reader.result;}$('settings-submit').disabled=false;};
  reader.onerror=()=>{$('settings-error').textContent='The image could not be read.';$('settings-error').hidden=false;$('settings-submit').disabled=false;};
  reader.readAsDataURL(file);
});
$('settings-form').addEventListener('submit',async event=>{
  event.preventDefault();$('settings-submit').disabled=true;$('settings-error').hidden=true;
  try{
    const body={username:$('settings-username').value,reset_avatar:state.resetAvatar,reset_cover:state.resetCover,details:{section_order:state.sectionOrder,country:$('settings-country').value,location:$('settings-location').value,interests:$('settings-interests').value,occupation:$('settings-occupation').value,twitter:$('settings-twitter').value,website:$('settings-website').value,discord:$('settings-discord').value,join_date:$('settings-join').value||'',about:$('settings-about').value,devices:[...document.querySelectorAll('input[name=device]:checked')].map(input=>input.value)}};
    if(state.avatarUpload)body.avatar=state.avatarUpload;
    if(state.coverUpload)body.cover=state.coverUpload;
    const result=await api('/site/settings',body);
    state.session=await api('/site/session');state.avatarVersion=Date.now();renderSession();
    await loadProfile(result.name,true);$('settings-dialog').close();
  }catch(error){$('settings-error').textContent=error.message;$('settings-error').hidden=false;}
  finally{$('settings-submit').disabled=false;}
});

"AD AE AF AG AI AL AM AO AQ AR AS AT AU AW AX AZ BA BB BD BE BF BG BH BI BJ BL BM BN BO BQ BR BS BT BV BW BY BZ CA CC CD CF CG CH CI CK CL CM CN CO CR CU CV CW CX CY CZ DE DJ DK DM DO DZ EC EE EG EH ER ES ET FI FJ FK FM FO FR GA GB GD GE GF GG GH GI GL GM GN GP GQ GR GS GT GU GW GY HK HM HN HR HT HU ID IE IL IM IN IO IQ IR IS IT JE JM JO JP KE KG KH KI KM KN KP KR KW KY KZ LA LB LC LI LK LR LS LT LU LV LY MA MC MD ME MF MG MH MK ML MM MN MO MP MQ MR MS MT MU MV MW MX MY MZ NA NC NE NF NG NI NL NO NP NR NU NZ OM PA PE PF PG PH PK PL PM PN PR PS PT PW PY QA RE RO RS RU RW SA SB SC SD SE SG SH SI SJ SK SL SM SN SO SR SS ST SV SX SY SZ TC TD TF TG TH TJ TK TL TM TN TO TR TT TV TW TZ UA UG UM US UY UZ VA VC VE VG VI VN VU WF WS YE YT ZA ZM ZW".split(' ').sort((a,b)=>countryName(a).localeCompare(countryName(b))).forEach(code=>{const option=document.createElement('option');option.value=code;option.textContent=countryName(code);$('settings-country').append(option);});
$('settings-country').addEventListener('change',updateSettingsCountryFlag);

$('edit-about').addEventListener('click',async()=>{await openSettings();if(!$('settings-submit').disabled)$('settings-about').focus();});

function renderRankHover19(box,data,pts){
  box.innerHTML=`<svg viewBox="0 0 600 70" preserveAspectRatio="none" role="img" aria-label="Recorded osu!daily rank history"><polyline points="${pts.map(q=>q.join(',')).join(' ')}" fill="none" stroke="#ffcc22" stroke-width="2" vector-effect="non-scaling-stroke"/></svg><div class="y19-hv" hidden><span class="y19-hv__line"></span><span class="y19-hv__dot"></span><div class="y19-hv__tip"></div></div>`;
  const hv=box.querySelector('.y19-hv'),line=hv.querySelector('.y19-hv__line'),dot=hv.querySelector('.y19-hv__dot'),tip=hv.querySelector('.y19-hv__tip');
  const real=data.length===1?[data[0]]:data;
  const ptsReal=data.length===1?[pts[pts.length-1]]:pts;
  const ago=d=>{const t=Date.parse(d);if(isNaN(t))return '';const n=Math.floor((Date.now()-t)/86400000);return n<=0?'today':n===1?'1 day ago':n+' days ago';};
  box.onmousemove=e=>{
    const r=box.getBoundingClientRect(),fx=(e.clientX-r.left)/r.width*600;
    let bi=0,bd=1e9;ptsReal.forEach((q,i)=>{const dd=Math.abs(q[0]-fx);if(dd<bd){bd=dd;bi=i;}});
    const q=ptsReal[bi],xp=q[0]/600*100,yp=q[1]/70*100;
    hv.hidden=false;line.style.left=xp+'%';dot.style.left=xp+'%';dot.style.top=yp+'%';
    tip.innerHTML=`<b>Global Ranking</b> #${num(real[bi][1])}<span>${esc(ago(real[bi][0]))}</span>`;
    const left=xp>60;tip.style.left=left?'auto':`calc(${xp}% + 14px)`;tip.style.right=left?`calc(${100-xp}% + 14px)`:'auto';
  };
  box.onmouseleave=()=>{hv.hidden=true;};
}
function renderPerformanceGraphNext(data){
  const box=$('rank-history');
  if(!data.length){box.textContent='';return;}
  const min=Math.min(...data.map(x=>x[1])),max=Math.max(...data.map(x=>x[1]));
  const pts=data.map((x,i)=>[data.length===1?596:i*596/(data.length-1),max===min?35:12+(x[1]-min)/(max-min)*46]);
  if(pts.length===1)pts.unshift([0,pts[0][1]]);
  const last=pts[pts.length-1];
  const label=data.length===1?`#${num(data[0][1])}`:`#${num(data[0][1])} → #${num(data[data.length-1][1])}`;
  if(T19){renderRankHover19(box,data,pts);return;}
  box.innerHTML=`<svg viewBox="0 0 600 70" preserveAspectRatio="none" role="img" aria-label="Recorded osu!daily rank history"><title>${esc(label)}</title><polyline points="${pts.map(q=>q.join(',')).join(' ')}" fill="none" stroke="#ffcc22" stroke-width="1.5" vector-effect="non-scaling-stroke"/></svg><span class="nx-rank-dot" style="left:${(last[0]/600*100).toFixed(2)}%;top:${(last[1]/70*100).toFixed(2)}%"></span>`;
  const gr=$('global-rank'),cr=$('country-rank');
  const ptsReal=data.length===1?[last]:pts, dataReal=data;
  const ago=d=>{const t=Date.parse(d);if(isNaN(t))return '';const n=Math.floor((Date.now()-t)/86400000);return n<=0?'today':n===1?'1 day ago':n+' days ago';};
  let saved=null,hv=null;
  box.onmousemove=e=>{
    const r=box.getBoundingClientRect(),fx=(e.clientX-r.left)/r.width*600;
    let bi=0,bd=1e9;ptsReal.forEach((q,k)=>{const dd=Math.abs(q[0]-fx);if(dd<bd){bd=dd;bi=k;}});
    if(!saved)saved=[gr.textContent,cr.textContent];
    if(!hv){hv=document.createElement('span');hv.className='nx-rank-dot nx-rank-dot--hv';box.appendChild(hv);}
    hv.style.left=(ptsReal[bi][0]/600*100).toFixed(2)+'%';hv.style.top=(ptsReal[bi][1]/70*100).toFixed(2)+'%';
    gr.textContent='#'+num(dataReal[bi][1]);cr.textContent=ago(dataReal[bi][0]);
  };
  box.onmouseleave=()=>{if(saved){gr.textContent=saved[0];cr.textContent=saved[1];saved=null;}if(hv){hv.remove();hv=null;}};
}
const RANKED_PLAYERS_ESTIMATE=3000000;
function globalRankTier(rank){
  if(!rank||rank<=0)return null;
  if(rank<=100)return 'lustrous';
  const pct=rank/RANKED_PLAYERS_ESTIMATE;
  return pct<0.0005?'radiant':pct<0.0015?'rhodium':pct<0.005?'platinum':pct<0.015?'gold':pct<0.05?'silver':pct<0.15?'bronze':pct<0.5?'iron':null;
}
function formatHighestDate(dateStr){
  if(!dateStr)return'';
  const parts=String(dateStr).split('T')[0].split('-');
  if(parts.length===3){
    const months=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
    const month=months[Number(parts[1])-1]||parts[1];
    return`${Number(parts[2])} ${month} ${parts[0]}`;
  }
  return dateStr;
}

function getHighestRank(p){
  if(p.rank_highest)return p.rank_highest;
  let best=null;
  if(p.rank_history&&p.rank_history.length){
    for(const [date,r] of p.rank_history){
      if(r>0&&(!best||r<best.rank)){
        best={rank:r,updated_at:date};
      }
    }
  }
  if(p.global_rank&&(!best||p.global_rank<best.rank)){
    best={rank:p.global_rank,updated_at:new Date().toISOString().slice(0,10)};
  }
  return best;
}

function getMonotonePath(pts){
  const n=pts.length;
  if(!n)return'';
  if(n===1)return`M 0,${pts[0].y.toFixed(2)} L 600,${pts[0].y.toFixed(2)}`;
  if(n===2)return`M ${pts[0].x.toFixed(2)},${pts[0].y.toFixed(2)} L ${pts[1].x.toFixed(2)},${pts[1].y.toFixed(2)}`;
  const dx=[],dy=[],s=[];
  for(let i=0;i<n-1;i++){
    const h=pts[i+1].x-pts[i].x,v=pts[i+1].y-pts[i].y;
    dx.push(h);dy.push(v);s.push(h!==0?v/h:0);
  }
  const t=new Array(n).fill(0);
  for(let i=1;i<n-1;i++){
    const s0=s[i-1],s1=s[i],h0=dx[i-1],h1=dx[i];
    if(s0*s1<=0){t[i]=0;}
    else{
      const p=(s0*h1+s1*h0)/(h0+h1);
      const sign=s0<0?-1:1;
      t[i]=2*sign*Math.min(Math.abs(s0),Math.abs(s1),0.5*Math.abs(p));
    }
  }
  t[0]=(3*s[0]-t[1])/2;
  t[n-1]=(3*s[n-2]-t[n-2])/2;
  let d=`M ${pts[0].x.toFixed(2)},${pts[0].y.toFixed(2)}`;
  for(let i=0;i<n-1;i++){
    const x0=pts[i].x,y0=pts[i].y,x1=pts[i+1].x,y1=pts[i+1].y;
    const h=(x1-x0)/3;
    const cp1x=x0+h,cp1y=y0+h*t[i],cp2x=x1-h,cp2y=y1-h*t[i+1];
    d+=` C ${cp1x.toFixed(2)},${cp1y.toFixed(2)} ${cp2x.toFixed(2)},${cp2y.toFixed(2)} ${x1.toFixed(2)},${y1.toFixed(2)}`;
  }
  return d;
}

function renderPerformanceGraph(){
  const cutoff=Date.now()-90*86400000;
  const data=(state.profile.rank_history||[]).filter(x=>{const t=Date.parse(x[0]);return isNaN(t)||t>=cutoff;}).slice(-90);
  if(NEXT){renderPerformanceGraphNext(data);return;}
  if(!data.length){$('rank-history').textContent='';return;}
  const min=Math.min(...data.map(x=>x[1])),max=Math.max(...data.map(x=>x[1]));
  const pts=data.map((x,i)=>({
    x:data.length===1?300:(i*600)/(data.length-1),
    y:max===min?35:10+((x[1]-min)/(max-min))*52
  }));
  const pathD=getMonotonePath(pts);
  $('rank-history').innerHTML=`<div class="rank-chart-wrap"><svg class="rank-chart-svg" viewBox="0 0 600 75" preserveAspectRatio="none" role="img" aria-label="Recorded osu!daily rank history"><path d="${pathD}" fill="none" stroke="#ffcc22" stroke-width="2" vector-effect="non-scaling-stroke" stroke-linecap="round" stroke-linejoin="round"/></svg><div class="rank-chart-hover" hidden><div class="rank-chart-hover__line"></div><div class="rank-chart-hover__circle"></div><div class="rank-chart-hover__tooltip"></div></div></div><span class="performance-caption">osu!daily rank history · ${data.length===1?'tracking started today':esc(data[0][0])+' – '+esc(data[data.length-1][0])}</span>`;

  const wrap=$('rank-history').querySelector('.rank-chart-wrap');
  const hover=wrap.querySelector('.rank-chart-hover');
  const hoverLine=wrap.querySelector('.rank-chart-hover__line');
  const hoverCircle=wrap.querySelector('.rank-chart-hover__circle');
  const tooltip=wrap.querySelector('.rank-chart-hover__tooltip');

  function formatRankTime(dateStr,idx,total){
    if(!dateStr)return'now';
    const parts=String(dateStr).split('-');
    if(parts.length===3){
      const d=new Date(Number(parts[0]),Number(parts[1])-1,Number(parts[2]));
      const now=new Date();
      const today=new Date(now.getFullYear(),now.getMonth(),now.getDate());
      const diffDays=Math.round((today-d)/86400000);
      if(diffDays<=0)return'now';
      if(diffDays===1)return'1 day ago';
      return`${diffDays} days ago`;
    }
    const daysAgo=total-1-idx;
    if(daysAgo<=0)return'now';
    if(daysAgo===1)return'1 day ago';
    return`${daysAgo} days ago`;
  }

  function updateHover(clientX){
    const rect=wrap.getBoundingClientRect();
    if(!rect.width)return;
    const relX=Math.max(0,Math.min(rect.width,clientX-rect.left));
    const ratio=relX/rect.width;
    const idx=Math.max(0,Math.min(data.length-1,Math.round(ratio*(data.length-1))));
    const pt=pts[idx];
    const item=data[idx];
    const pctX=(pt.x/600)*100;
    const pctY=(pt.y/75)*100;

    hover.hidden=false;
    hoverLine.style.left=`${pctX}%`;
    hoverCircle.style.left=`${pctX}%`;
    hoverCircle.style.top=`${pctY}%`;
    tooltip.innerHTML=`<div class="rank-chart-hover__title"><strong>Global Ranking</strong> #${num(item[1])}</div><div class="rank-chart-hover__time">${formatRankTime(item[0],idx,data.length)}</div>`;
    tooltip.style.left=`${pctX}%`;
    tooltip.style.top=`${pctY}%`;
    tooltip.style.marginTop='-12px';
    tooltip.style.transform=pctX<18?'translate(0, -100%)':pctX>82?'translate(-100%, -100%)':'translate(-50%, -100%)';
  }

  wrap.addEventListener('mousemove',e=>updateHover(e.clientX));
  wrap.addEventListener('mouseleave',()=>{hover.hidden=true;});
  wrap.addEventListener('touchmove',e=>{if(e.touches&&e.touches[0])updateHover(e.touches[0].clientX);},{passive:true});
  wrap.addEventListener('touchend',()=>{hover.hidden=true;});
}
function renderSectionOrder(){
  $('section-order-list').innerHTML=state.sectionOrder.map((id,i)=>`<div class="section-order-row"><span>${esc(document.querySelector('.section-tabs a[href="#'+id+'"]').textContent)}</span><button type="button" data-order="${i}" data-direction="-1" aria-label="Move ${esc(id)} up" ${i===0?'disabled':''}>↑</button><button type="button" data-order="${i}" data-direction="1" aria-label="Move ${esc(id)} down" ${i===state.sectionOrder.length-1?'disabled':''}>↓</button></div>`).join('');
}
$('section-order-list').addEventListener('click',event=>{
  const button=event.target.closest('[data-order]');if(!button)return;
  const i=Number(button.dataset.order),j=i+Number(button.dataset.direction);
  if(j<0||j>=state.sectionOrder.length)return;
  [state.sectionOrder[i],state.sectionOrder[j]]=[state.sectionOrder[j],state.sectionOrder[i]];renderSectionOrder();
});
$('profile-content').addEventListener('click',async event=>{
  const button=event.target.closest('[data-delete-score]');if(!button)return;
  if(!confirm('Delete this score from your profile? Your pp and statistics will be recalculated. A recovery copy is retained on the server.'))return;
  button.disabled=true;
  try{await api('/site/scores/delete',{id:button.dataset.deleteScore});await loadProfile(state.name);}
  catch(error){showMessage(error.message);button.disabled=false;}
});

async function initThemePicker(){
  const sel=$('site-theme'); if(!sel) return;
  const box=sel.closest('.theme-picker');
  try{
    const d=await api('/site/themes');
    if(!d.themes||d.themes.length<2){ if(box) box.hidden=true; return; }
    sel.innerHTML=d.themes.map(t=>`<option value="${esc(t.id)}" title="${esc(t.description||'')}">${esc(t.name)}</option>`).join('');
    sel.value=d.current;
    sel.addEventListener('change',()=>{
      document.cookie=`site_theme=${encodeURIComponent(sel.value)}; path=/; max-age=31536000; SameSite=Lax`;
      location.reload();
    });
  }catch(e){ if(box) box.hidden=true; }
}
initThemePicker();
(function initPpSystemPicker(){
  const sel=$('pp-system'); if(!sel) return;
  if([...sel.options].some(o=>o.value===state.ppsys)) sel.value=state.ppsys; else state.ppsys='live';
  sel.addEventListener('change',()=>{
    state.ppsys=sel.value;
    try{localStorage.setItem('site_ppsys',state.ppsys);}catch(e){}
    if(state.name) loadProfile(state.name,false,true);
  });
  const follow=$('pp-rank-follow');
  if(follow){
    follow.checked=state.ppfollow;
    follow.addEventListener('change',()=>{
      state.ppfollow=follow.checked;
      try{localStorage.setItem('site_ppfollow',state.ppfollow?'1':'0');}catch(e){}
      if(state.name&&state.ppsys!=='live') loadProfile(state.name,false,true);
    });
  }
  let timer=null;
  state.showPpSystemNote=(p)=>{
    const note=$('pp-system-note'); if(!note) return;
    clearTimeout(timer);
    if(!p||!p.pp_system||p.pp_system==='live'){note.hidden=true;return;}
    const miss=p.pp_system_missing||0;
    note.hidden=false;
    note.textContent=miss?`${miss} osu! play(s) hidden: pp for this system is ${p.pp_system_filling?'being calculated…':'not calculated yet (run !recalc '+p.pp_system+' in game)'}`:'pp from the selected system';
    if(miss&&p.pp_system_filling&&state.name){timer=setTimeout(()=>{ if(state.ppsys===p.pp_system) loadProfile(state.name,false,true); },8000);}
  };
})();
(function initEraPicker(){
  const sel=$('rank-era'); if(!sel) return;
  sel.value=state.era;
  sel.addEventListener('change',()=>{
    state.era=sel.value;
    try{localStorage.setItem('site_era',state.era);}catch(e){}
    if(state.name) loadProfile(state.name,false,true);
    if(state.session&&state.session.csrf) api('/site/ladder_era',{era:state.era}).catch(()=>{});
  });
  const box=$('ladder-info'), txt=$('ladder-text'), btn=$('ladder-refresh');
  if(!box) return;
  const show=(i)=>{ if(!i||!i.available){box.hidden=true;return;} box.hidden=false;
    txt.textContent=`Ladder data through ${i.last_date}`; box.title=`${i.path} (${i.source}); auto refresh ${i.auto_refresh?'on':'off'}`; };
  let synced=false;
  const load=()=>fetch('/site/ladder_info',{cache:'no-store'}).then(r=>r.json()).then(i=>{
    show(i);
    if(i&&Array.isArray(i.eras)&&i.eras.length){
      const keep=state.era;
      sel.innerHTML='<option value="latest">Latest (osu!daily)</option>'+i.eras.map(e=>`<option value="${esc(e.id)}">${esc(e.label)}</option>`).join('');
      sel.value=[...sel.options].some(o=>o.value===keep)?keep:'latest';
      if(sel.value!==keep){state.era=sel.value;}
    }
    if(!synced&&i&&i.era&&i.era!==state.era){
      state.era=i.era; sel.value=i.era; try{localStorage.setItem('site_era',i.era);}catch(e){}
      if(state.name) loadProfile(state.name,false,true);
    }
    synced=true;
  }).catch(()=>{});
  load();
  if(btn) btn.addEventListener('click',async()=>{
    if(!state.session||!state.session.csrf){txt.textContent='Sign in to refresh the ladder data';return;}
    btn.disabled=true; txt.textContent='Refreshing…';
    try{ await api('/site/ladder_refresh',{}); await load(); if(state.era!=='latest'&&state.name) loadProfile(state.name,false,true); }
    catch(e){ txt.textContent='Refresh failed: '+((e&&e.message)||e); }
    btn.disabled=false;
  });
})();

document.addEventListener('click', e => {
  if (e.defaultPrevented || e.button !== 0 || e.target.closest('a,button,input,select,label,.popup-menu')) return;
  const row = e.target.closest('[data-score]');
  const id = row && row.getAttribute('data-score');
  if (id && /^\d+$/.test(id)) location.href = `/scores/${id}`;
});
