(function(){
  var OSU='https://osu.ppy.sh';
  var root=document.getElementById('bm-root');
  var MODES=[['osu','osu!',0],['taiko','osu!taiko',1],['fruits','osu!catch',2],['mania','osu!mania',3]];
  var MODE_KEY=['osu','taiko','fruits','mania'];
  function esc(s){return String(s==null?'':s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];});}
  function num(n){try{return Number(n||0).toLocaleString();}catch(e){return String(n||0);}}
  function $(sel,el){return (el||document).querySelector(sel);}
  function getJSON(u){return fetch(u,{credentials:'same-origin'}).then(function(r){return r.json().catch(function(){return {};}).then(function(j){if(!r.ok)throw new Error(j&&j.error||String(r.status));return j;});});}
  var STOPS=[[0.1,[66,144,251]],[1.25,[79,192,255]],[2,[79,255,213]],[2.5,[124,255,79]],[3.3,[246,240,92]],[4.2,[255,128,104]],[4.9,[255,78,111]],[5.8,[198,69,184]],[6.7,[101,99,222]],[7.7,[24,21,142]],[9,[0,0,0]]];
  function diffColour(r){
    r=Number(r)||0;if(r<0.1)return '#aaaaaa';if(r>=9)return '#000000';
    for(var i=1;i<STOPS.length;i++){if(r<=STOPS[i][0]){var a=STOPS[i-1],b=STOPS[i],t=(r-a[0])/(b[0]-a[0]),c=[0,1,2].map(function(k){return Math.round(a[1][k]+(b[1][k]-a[1][k])*t);});return 'rgb('+c.join(',')+')';}}
    return '#000';
  }
  var STATUS={ranked:'Ranked',approved:'Approved',qualified:'Qualified',loved:'Loved',pending:'Pending',wip:'WIP',graveyard:'Graveyard'};
  function statusStyle(s){var k=s==='approved'?'ranked':s;return '--bg-hsl:var(--beatmapset-'+k+'-bg-hsl,var(--hsl-b2));--colour:var(--beatmapset-'+k+'-colour,#fff)';}
  function modeIcon(m,cls){return '<span class="fal fa-extra-mode-'+MODE_KEY[m||0]+' '+(cls||'')+'"></span>';}
  function cover(id,kind){var u='https://assets.ppy.sh/beatmaps/'+id+'/covers/'+kind+'.jpg';return '--bg:url(\''+u+'\');--bg-2x:url(\''+u.replace('.jpg','@2x.jpg')+'\')';}
  function fmtDate(iso){var t=Date.parse(iso);if(isNaN(t))return '';var d=new Date(t);return d.getUTCDate()+' '+['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'][d.getUTCMonth()]+' '+d.getUTCFullYear();}
  function fmtLen(s){s=Math.round(Number(s)||0);var m=Math.floor(s/60),r=s%60;return m+':'+(r<10?'0':'')+r;}
  function fix(n,d){n=Number(n);return isFinite(n)?n.toFixed(d).replace(/\.0+$/,'').replace(/(\.\d*?)0+$/,'$1'):'-';}
  function ago(ts){
    var t=Number(ts)||0;if(t>1e12)t/=1000;if(!t)return '';
    var d=Math.max(0,Math.round(Date.now()/1000-t)),u=[['year',31536000],['month',2592000],['day',86400],['hour',3600],['minute',60]];
    for(var i=0;i<u.length;i++)if(d>=u[i][1]){var n=Math.floor(d/u[i][1]);return n+' '+u[i][0]+(n>1?'s':'')+' ago';}
    return 'just now';
  }

  var session={user:null};
  var NEXT=document.body.classList.contains('theme-next');
  var Y19=document.body.classList.contains('theme-2019');
  function nav(){if(NEXT||Y19)return '';return window.SiteNav?SiteNav.html(session.s||{user:session.user},'beatmaps'):'';}
  var NX_DROP="<div class=\"nx-drop\" aria-label=\"Site navigation\"><nav class=\"nx-nav\"><div class=\"nx-nav__item\"><a href=\"https://osu.ppy.sh/home/news\" target=\"_blank\" rel=\"noreferrer\">home</a><div class=\"nx-nav__sub\"><a href=\"https://osu.ppy.sh/home/news\" target=\"_blank\" rel=\"noreferrer\">news</a><a href=\"/home/friends\">friends</a><a href=\"https://osu.ppy.sh/home/changelog\" target=\"_blank\" rel=\"noreferrer\">changelog</a><a href=\"https://osu.ppy.sh/home/download\" target=\"_blank\" rel=\"noreferrer\">download</a><a href=\"https://osu.ppy.sh/home/search\" target=\"_blank\" rel=\"noreferrer\">search</a></div></div><div class=\"nx-nav__item\"><a href=\"https://osu.ppy.sh/wiki/Welcome\" target=\"_blank\" rel=\"noreferrer\">help</a><div class=\"nx-nav__sub\"><a href=\"https://osu.ppy.sh/wiki/Welcome\" target=\"_blank\" rel=\"noreferrer\">wiki</a><a href=\"https://osu.ppy.sh/wiki/FAQ\" target=\"_blank\" rel=\"noreferrer\">faq</a><a href=\"https://osu.ppy.sh/wiki/Help_Center\" target=\"_blank\" rel=\"noreferrer\">support</a></div></div><div class=\"nx-nav__item\"><a href=\"https://osu.ppy.sh/rankings/osu/performance\" target=\"_blank\" rel=\"noreferrer\">rankings</a><div class=\"nx-nav__sub\"><a href=\"https://osu.ppy.sh/rankings/osu/performance\" target=\"_blank\" rel=\"noreferrer\">performance</a><a href=\"https://osu.ppy.sh/p/chart\" target=\"_blank\" rel=\"noreferrer\">charts</a><a href=\"https://osu.ppy.sh/rankings/osu/score\" target=\"_blank\" rel=\"noreferrer\">score</a><a href=\"https://osu.ppy.sh/rankings/osu/country\" target=\"_blank\" rel=\"noreferrer\">country</a><a href=\"https://osu.ppy.sh/p/kudosu\" target=\"_blank\" rel=\"noreferrer\">kudosu</a></div></div><div class=\"nx-nav__item nx-nav__item--active\"><a href=\"/beatmapsets\">beatmaps</a><div class=\"nx-nav__sub\"><a href=\"/beatmapsets\">listing</a><a href=\"https://osu.ppy.sh/beatmaps/artists\" target=\"_blank\" rel=\"noreferrer\">featured artists</a><a href=\"https://osu.ppy.sh/beatmaps/packs\" target=\"_blank\" rel=\"noreferrer\">packs</a></div></div><div class=\"nx-nav__item\"><a href=\"https://osu.ppy.sh/community/forums\" target=\"_blank\" rel=\"noreferrer\">community</a><div class=\"nx-nav__sub\"><a href=\"https://osu.ppy.sh/community/forums\" target=\"_blank\" rel=\"noreferrer\">forum</a><a href=\"https://osu.ppy.sh/community/contests\" target=\"_blank\" rel=\"noreferrer\">contests</a><a href=\"https://osu.ppy.sh/community/tournaments\" target=\"_blank\" rel=\"noreferrer\">tournaments</a><a href=\"https://osu.ppy.sh/community/livestreams\" target=\"_blank\" rel=\"noreferrer\">live</a><a href=\"https://discord.gg/ppy\" target=\"_blank\" rel=\"noreferrer\">osu!dev</a></div></div><div class=\"nx-nav__item\"><a href=\"https://osu.ppy.sh/store/listing\" target=\"_blank\" rel=\"noreferrer\">store</a><div class=\"nx-nav__sub\"><a href=\"https://osu.ppy.sh/store/listing\" target=\"_blank\" rel=\"noreferrer\">listing</a><a href=\"https://osu.ppy.sh/store/cart\" target=\"_blank\" rel=\"noreferrer\">cart</a></div></div><a class=\"nx-nav__lang\" href=\"/\" target=\"_blank\" rel=\"noreferrer\"><img src=\"https://osu.ppy.sh/assets/images/flags/1f1ec-1f1e7.svg\" alt=\"\" width=\"20\" height=\"14\"> English</a></nav><div class=\"nx-drop__social\"><a href=\"https://osu.ppy.sh/home/support\" target=\"_blank\" rel=\"noreferrer\" aria-label=\"Support osu!\"><svg viewBox=\"0 0 24 24\" width=\"16\" height=\"16\" fill=\"#fff\"><path d=\"M12 21s-7.5-4.6-9.6-9.3C.8 8 2.6 4.5 6 4.5c2 0 3.4 1 4.4 2.5.4.5.8.5 1.2 0C12.600 5.500 14 4.500 16 4.500c3.400 0 5.200 3.500 3.600 7.200C19.500 16.400 12 21 12 21z\"/></svg></a><a href=\"https://twitter.com/osugame\" target=\"_blank\" rel=\"noreferrer\" aria-label=\"Twitter\"><svg viewBox=\"0 0 24 24\" width=\"16\" height=\"16\" fill=\"#fff\"><path d=\"M22 5.900c-.7.300-1.500.500-2.300.600.800-.500 1.500-1.300 1.800-2.200-.8.500-1.700.800-2.600 1-.8-.8-1.800-1.300-3-1.300-2.300 0-4.100 1.800-4.100 4.100 0 .3 0 .6.100.9C7.500 8.800 4.500 7.200 2.500 4.700c-.4.600-.6 1.300-.6 2 0 1.400.7 2.700 1.800 3.400-.7 0-1.300-.2-1.800-.5 0 2 1.400 3.700 3.300 4.100-.6.200-1.200.2-1.800.1.500 1.600 2 2.800 3.800 2.800-1.400 1.100-3.200 1.800-5.100 1.800H1c1.800 1.200 4 1.800 6.300 1.800 7.500 0 11.700-6.300 11.700-11.700v-.5c.8-.6 1.500-1.300 2-2.100z\"/></svg></a><a href=\"https://www.facebook.com/osugame\" target=\"_blank\" rel=\"noreferrer\" aria-label=\"Facebook\"><svg viewBox=\"0 0 24 24\" width=\"16\" height=\"16\" fill=\"#fff\"><path d=\"M14 8V6.300c0-.7.200-1.100 1.300-1.100H17V2h-2.700C11.200 2 10 3.800 10 6.100V8H7.500v3.200H10V22h4V11.200h2.700L17 8z\"/></svg></a></div></div>";
  function nextHeader(title){
    var u=session.user,av=u?'/site/avatar?name='+encodeURIComponent(u):'/site/static/vendor/avatar-guest@2x.01495bc4.png';
    return '<header class="local-header"><div class="nav2-header__triangles"></div><nav class="local-nav"><a href="/" class="local-logo" aria-label="osu! home"><img src="/site/static/themes/next/img/osu-text.svg" alt="osu!"></a>'+
      '<a class="account-button" href="'+(u?'/users/'+encodeURIComponent(u):'/')+'"><img class="account-avatar" src="'+av+'" alt="" onerror="this.onerror=null;this.src=\'/site/static/vendor/avatar-guest@2x.01495bc4.png\'"><span id="account-name">'+esc(u||'Guest')+'</span></a></nav>'+
      '<div class="page-heading"><a href="/beatmapsets" class="crumb">beatmaps</a><i class="crumb-sep fas fa-angle-right" aria-hidden="true"></i><span class="crumb">'+esc(title.replace(/^beatmap /,''))+'</span></div>'+NX_DROP+'</header>';
  }
  function pageHeader(title,tabs,right,variant,bg){
    if(NEXT)return '<div class="nxb-top">'+nextHeader(title)+'</div>';
    if(Y19&&window.Y19Header)return Y19Header.html(session.user,'beatmaps')+'<div class="y19l-spacer"></div>';
    var bar=tabs&&tabs.length?'<div class="header-v4__container"><div class="header-v4__content"><div class="header-v4__row header-v4__row--bar"><ul class="header-nav-v4 header-nav-v4--list">'+
      tabs.map(function(t){return '<li class="header-nav-v4__item"><a class="header-nav-v4__link'+(t[2]?' header-nav-v4__link--active':'')+'" href="'+t[1]+'"'+(t[1].charAt(0)==='/'?'':' target="_blank" rel="noreferrer"')+'><span class="fake-bold" data-content="'+esc(t[0])+'">'+esc(t[0])+'</span></a></li>';}).join('')+
      '</ul>'+(right?'<div class="bm-bar-right">'+right+'</div>':'')+'</div></div></div>':'';
    return '<div class="bm-head"><div class="header-v4 header-v4--'+(variant||'beatmapsets')+'">'+nav()+
      '<div class="header-v4__container header-v4__container--main"><div class="header-v4__bg-container"><div class="header-v4__bg"'+(bg?' style="background-image:url(&quot;'+bg+'&quot;)"':'')+'></div></div>'+
      '<div class="header-v4__content"><div class="header-v4__row header-v4__row--title"><div class="header-v4__icon"></div><div class="header-v4__title">'+esc(title)+'</div></div></div></div>'+bar+'</div></div>';
  }
  function footer(){return '<div class="bm-foot"><a href="'+OSU+'/wiki/en/Rules" target="_blank" rel="noreferrer">Rules</a><a href="'+OSU+'/legal/terms" target="_blank" rel="noreferrer">Terms</a><a href="'+OSU+'/legal/privacy" target="_blank" rel="noreferrer">Privacy</a><a href="https://status.ppy.sh" target="_blank" rel="noreferrer">Server Status</a></div>';}

  var audio=null,playingBtn=null;
  function togglePreview(btn,id){
    if(playingBtn){playingBtn.classList.remove('bm-playing');playingBtn.querySelector('.fas')&&playingBtn.querySelector('.fas').classList.add('fa-play');playingBtn.querySelector('.fas')&&playingBtn.querySelector('.fas').classList.remove('fa-pause');}
    if(audio){audio.pause();audio=null;}
    if(playingBtn===btn){playingBtn=null;return;}
    playingBtn=btn;audio=new Audio('https://b.ppy.sh/preview/'+id+'.mp3');audio.volume=.4;
    var ic=btn.querySelector('.fas');if(ic){ic.classList.remove('fa-play');ic.classList.add('fa-pause');}
    audio.onended=function(){togglePreviewReset();};audio.onerror=function(){togglePreviewReset();};
    audio.play().catch(function(){togglePreviewReset();});
  }
  function togglePreviewReset(){if(playingBtn&&playingBtn.querySelector('.fas')){playingBtn.querySelector('.fas').classList.add('fa-play');playingBtn.querySelector('.fas').classList.remove('fa-pause');}playingBtn=null;audio=null;}

  var L={q:'',m:'',s:'leaderboard',sort:'ranked_desc',nsfw:'0',size:'normal',page:0,items:[],more:false,busy:false,source:'',expanded:false};
  var STATUSES=[['leaderboard','Has Leaderboard'],['ranked','Ranked'],['qualified','Qualified'],['loved','Loved'],['pending','Pending'],['graveyard','Graveyard'],['any','Any']];
  var SORTS=[['title','Title','title_asc','title_desc'],['artist','Artist','artist_asc','artist_desc'],['ranked','Ranked','ranked_desc','ranked_asc'],['plays','Plays','plays_desc','plays_asc'],['favourites','Favourites','favourites_desc','favourites_asc'],['updated','Updated','updated_desc','updated_asc']];
  function readQuery(){
    var p=new URLSearchParams(location.search);
    L.q=p.get('q')||'';L.m=p.get('m')||'';L.s=p.get('s')||'leaderboard';L.sort=p.get('sort')||'ranked_desc';L.nsfw=p.get('nsfw')==='1'?'1':'0';
    try{L.size=localStorage.getItem('los_bm_size')==='extra'?'extra':'normal';}catch(e){}
  }
  function pushQuery(){
    var p=new URLSearchParams();
    if(L.q)p.set('q',L.q);if(L.m!=='')p.set('m',L.m);if(L.s!=='leaderboard')p.set('s',L.s);if(L.sort!=='ranked_desc')p.set('sort',L.sort);if(L.nsfw==='1')p.set('nsfw','1');
    var qs=p.toString();try{history.replaceState(null,'','/beatmapsets'+(qs?'?'+qs:''));}catch(e){}
  }
  function filterRow(title,key,opts,extra){
    return '<div class="beatmapsets-search-filter beatmapsets-search-filter--grid'+(extra||'')+'"><span class="beatmapsets-search-filter__header">'+title+'</span><div class="beatmapsets-search-filter__items">'+
      opts.map(function(o){return '<button type="button" class="beatmapsets-search-filter__item'+(String(L[key])===String(o[0])?' beatmapsets-search-filter__item--active':'')+'" data-f="'+key+'" data-v="'+esc(o[0])+'">'+esc(o[1])+'</button>';}).join('')+'</div></div>';
  }
  function nxSearch(){
    function grp(title,key,opts){return '<div class="nxl-filter"><span class="nxl-filter__h">'+title+'</span>'+opts.map(function(o){return '<button type="button" class="nxl-filter__i'+(String(L[key])===String(o[0])?' nxl-filter__i--on':'')+'" data-f="'+key+'" data-v="'+esc(o[0])+'">'+esc(o[1])+'</button>';}).join('')+'</div>';}
    return '<div class="nxl-search"><div class="nxl-search__box"><span class="fas fa-search"></span><input id="bm-q" type="search" placeholder="type in keywords..." autocomplete="off" value="'+esc(L.q)+'"></div>'+
      grp('Mode','m',[['','Any']].concat(MODES.map(function(m){return [String(m[2]),m[1]];})))+grp('Status','s',STATUSES)+grp('Explicit','nsfw',[['0','Hide'],['1','Show']])+'</div>';
  }
  function searchHtml(){
    if(NEXT)return nxSearch();
    return '<div class="osu-page osu-page--beatmapsets-search-header"><div class="bm-search'+(L.expanded?' bm-expanded':'')+'"><div class="beatmapsets-search"><div class="beatmapsets-search__cover"></div>'+
      '<div class="beatmapsets-search__input-container"><span class="beatmapsets-search__icon fas fa-search"></span><input id="bm-q" class="beatmapsets-search__input" type="search" placeholder="type in keywords..." autocomplete="off" value="'+esc(L.q)+'"></div>'+
      '<div class="beatmapsets-search__filter-grid">'+
      filterRow('General','s',STATUSES)+
      filterRow('Mode','m',[['','Any']].concat(MODES.map(function(m){return [String(m[2]),m[1]];})))+
      '<div class="bm-adv">'+filterRow('Explicit Content','nsfw',[['0','Hide'],['1','Show']])+'</div>'+
      '</div><button type="button" class="bm-expand" id="bm-expand">'+(L.expanded?'LESS':'MORE')+' SEARCH OPTIONS <span class="fas fa-angle-'+(L.expanded?'up':'down')+'"></span></button></div></div></div>';
  }
  function sortHtml(){
    return '<div class="osu-page"><div class="beatmapsets"><div class="bm-toolbar"><div class="sort sort--beatmapsets"><div class="sort__items"><span class="sort__item sort__item--title">Sort by</span>'+
      SORTS.map(function(s){var on=L.sort===s[2]||L.sort===s[3],desc=L.sort===s[2];return '<a class="sort__item sort__item--button'+(on?' sort__item--active':'')+'" href="#" data-sort="'+s[0]+'">'+s[1]+'<span class="sort__item-arrow fas fa-caret-'+(on&&!desc&&s[0]!=='title'&&s[0]!=='artist'||on&&desc&&(s[0]==='title'||s[0]==='artist')?'up':'down')+'"></span></a>';}).join('')+
      '</div></div>'+(NEXT||Y19?'':'<div class="bm-sizes"><button type="button" data-size="normal" class="'+(L.size==='normal'?'on':'')+'" title="normal"><span class="fas fa-th-list"></span></button><button type="button" data-size="extra" class="'+(L.size==='extra'?'on':'')+'" title="extra"><span class="fas fa-th-large"></span></button></div>')+'</div>'+
      '<div class="bm-items" id="bm-items"></div><div id="bm-foot-area"></div></div></div>';
  }
  function dotsHtml(set){
    var by={};(set.beatmaps||[]).forEach(function(b){(by[b.mode||0]=by[b.mode||0]||[]).push(b);});
    return Object.keys(by).sort().map(function(m){
      var maps=by[m].slice().sort(function(a,b){return a.stars-b.stars;});
      var shown=maps.length>12?[]:maps;
      return '<span class="beatmapset-panel__extra-item beatmapset-panel__extra-item--dots">'+modeIcon(Number(m),'beatmapset-panel__beatmap-icon')+
        (shown.length?shown.map(function(b){return '<span class="beatmapset-panel__beatmap-dot" style="--bg:'+diffColour(b.stars)+'" title="'+esc(b.version)+' ('+fix(b.stars,2)+'★)"></span>';}).join(''):'<span class="beatmapset-panel__beatmap-count">'+maps.length+'</span>')+'</span>';
    }).join('');
  }
  function nextPanel(set){
    var id=set.id,href='/beatmapsets/'+id,by={};(set.beatmaps||[]).forEach(function(b){(by[b.mode||0]=by[b.mode||0]||[]).push(b);});
    var diffs=Object.keys(by).sort().map(function(m){return by[m].slice().sort(function(a,b){return a.stars-b.stars;}).slice(0,14).map(function(b){
      return '<span class="nxl-diff" style="--c:'+diffColour(b.stars)+'" title="'+esc(b.version)+' ('+fix(b.stars,2)+'★)">'+modeIcon(Number(m))+'</span>';}).join('')+(by[m].length>14?'<span class="nxl-diff-more">+'+(by[m].length-14)+'</span>':'');}).join('');
    return '<div class="nxl-item"><div class="nxl-panel"><div class="nxl-panel__panel">'+
      '<div class="nxl-panel__header"><a href="'+href+'" class="nxl-panel__thumb" style="'+cover(id,'card')+'"><div class="nxl-panel__title-artist-box"><div class="u-ellipsis-overflow nxl-panel__header-text nxl-panel__header-text--title">'+esc(set.title)+'</div><div class="u-ellipsis-overflow nxl-panel__header-text">'+esc(set.artist)+'</div></div>'+
      (Y19?'<span class="nxl-status nxl-status--cover" data-st="'+esc(set.status)+'">'+esc(STATUS[set.status]||set.status)+'</span>':'')+'<div class="nxl-panel__counts-box"><div class="nxl-panel__count"><span class="nxl-panel__count-number">'+num(set.plays)+'</span><span class="fas fa-play-circle fa-fw"></span></div><div class="nxl-panel__count"><span class="nxl-panel__count-number">'+num(set.favourites)+'</span><span class="fas fa-heart fa-fw"></span></div></div></a></div>'+
      '<div class="nxl-panel__content"><div class="nxl-panel__row"><div class="nxl-panel__mapper-source-box"><div class="u-ellipsis-overflow">mapped by <a href="/beatmapsets/'+id+'">'+esc(set.creator)+'</a></div>'+(set.source?'<div class="u-ellipsis-overflow">'+esc(set.source)+'</div>':'')+'</div>'+
      '<div class="nxl-panel__icons-box">'+(Y19?'':'<span class="nxl-status" data-st="'+esc(set.status)+'">'+esc(STATUS[set.status]||set.status)+'</span>')+'</div></div>'+
      '<div class="nxl-panel__difficulties">'+diffs+'</div></div>'+
      '<button type="button" class="nxl-panel__play" data-preview="'+id+'" aria-label="preview"><span class="fas fa-play"></span></button>'+
      '<a class="nxl-panel__dl" href="/d/'+id+'" title="download"><span class="fas fa-download"></span></a>'+
      '<div class="nxl-panel__shadow"></div></div></div></div>';
  }
  function panel(set){
    if(NEXT||Y19)return nextPanel(set);
    var id=set.id,href='/beatmapsets/'+id,date=fmtDate(set.ranked_date||set.submitted_date||set.last_updated);
    return '<div class="beatmapsets__item"><div class="beatmapset-panel beatmapset-panel--size-'+L.size+'" data-id="'+id+'">'+
      '<a class="bm-panel-link" href="'+href+'" aria-label="'+esc(set.title)+'"></a>'+
      '<div class="beatmapset-panel__cover-container"><div class="beatmapset-panel__cover-col beatmapset-panel__cover-col--play"><div class="beatmapset-cover beatmapset-cover--full" style="'+cover(id,'list')+'"></div></div>'+
      '<div class="beatmapset-panel__cover-col beatmapset-panel__cover-col--info"><div class="beatmapset-cover beatmapset-cover--full" style="'+cover(id,'card')+'"></div></div></div>'+
      '<div class="beatmapset-panel__content"><div class="beatmapset-panel__play-container"><button type="button" class="beatmapset-panel__play" data-preview="'+id+'" aria-label="preview"><span class="beatmapset-panel__play-icon fas fa-play"></span></button></div>'+
      '<div class="beatmapset-panel__info">'+
      '<div class="beatmapset-panel__info-row beatmapset-panel__info-row--title"><a class="beatmapset-panel__main-link u-ellipsis-overflow" href="'+href+'">'+esc(set.title)+'</a></div>'+
      '<div class="beatmapset-panel__info-row beatmapset-panel__info-row--artist"><a class="u-ellipsis-overflow" href="'+href+'">'+esc(set.artist)+'</a></div>'+
      (set.source?'<div class="beatmapset-panel__info-row beatmapset-panel__info-row--source"><span class="u-ellipsis-overflow">'+esc(set.source)+'</span></div>':'')+
      '<div class="beatmapset-panel__info-row beatmapset-panel__info-row--mapper"><span class="u-ellipsis-overflow">mapped by '+esc(set.creator)+'</span></div>'+
      '<div class="beatmapset-panel__info-row beatmapset-panel__info-row--stats"><span class="beatmapset-panel__stats-item beatmapset-panel__stats-item--play-count" title="play count"><span class="beatmapset-panel__stats-item-icon fas fa-play-circle"></span>'+num(set.plays)+'</span> '+
      '<span class="beatmapset-panel__stats-item beatmapset-panel__stats-item--favourite-count" title="favourites"><span class="beatmapset-panel__stats-item-icon fas fa-heart"></span>'+num(set.favourites)+'</span> '+
      (date?'<span class="beatmapset-panel__stats-item beatmapset-panel__stats-item--date"><span class="beatmapset-panel__stats-item-icon fas fa-clock"></span>'+esc(date)+'</span>':'')+'</div>'+
      '<div class="beatmapset-panel__info-row beatmapset-panel__info-row--extra"><span class="beatmapset-panel__extra-item"><span class="beatmapset-status beatmapset-status--panel" style="'+statusStyle(set.status)+'">'+esc(STATUS[set.status]||set.status)+'</span></span>'+dotsHtml(set)+'</div>'+
      '</div><div class="beatmapset-panel__menu-container"><div class="beatmapset-panel__menu"><a class="beatmapset-panel__menu-item" href="/d/'+id+'" title="download"><span class="fas fa-file-download"></span></a><a class="beatmapset-panel__menu-item" href="'+OSU+'/beatmapsets/'+id+'" target="_blank" rel="noreferrer" title="open on osu.ppy.sh"><span class="fas fa-external-link-alt"></span></a></div></div></div></div></div>';
  }
  function renderItems(){
    var el=$('#bm-items');if(!el)return;
    if(!L.items.length&&!L.busy){
      el.innerHTML='<div class="bm-empty">'+(L.error?esc(L.error)+'<br><br>':'')+(L.error?'':'No beatmaps found.<br><br>')+'<a href="'+OSU+'/beatmapsets'+(L.q?'?q='+encodeURIComponent(L.q):'')+'" target="_blank" rel="noreferrer">Open the official listing</a></div>';
      $('#bm-foot-area').innerHTML='';return;
    }
    if(NEXT||Y19){
      el.innerHTML='<div class="nxl-items">'+L.items.map(nextPanel).join('')+'</div>';
      var bg=$('.nxl-banner__bg');if(Y19&&L.items[0]){var cv=$('.beatmapsets-search__cover');if(cv&&!cv.getAttribute('data-set')){cv.setAttribute('data-set','1');cv.style.backgroundImage='url(\'https://assets.ppy.sh/beatmaps/'+L.items[0].id+'/covers/cover.jpg\')';}}if(bg&&L.items[0]&&!bg.getAttribute('data-set')){bg.setAttribute('data-set','1');bg.style.backgroundImage='url(\'https://assets.ppy.sh/beatmaps/'+L.items[0].id+'/covers/cover.jpg\')';}
      $('#bm-foot-area').innerHTML=(L.more?'<div class="nxl-more"><a href="#" class="nxl-more__link" id="bm-more">'+(L.busy?'loading...':'show more')+'</a></div>':'')+(L.source?'<div class="bm-src">Results from '+esc(L.source)+'</div>':'');
      return;
    }
    var rows='';for(var i=0;i<L.items.length;i+=2){rows+='<div class="beatmapsets__items-row">'+panel(L.items[i])+(L.items[i+1]?panel(L.items[i+1]):'')+'</div>';}
    el.innerHTML='<div class="beatmapsets__items">'+rows+'</div>';
    $('#bm-foot-area').innerHTML=(L.more?'<div class="bm-more"><button type="button" class="btn-osu-big btn-osu-big--rounded-thin" id="bm-more"><span class="btn-osu-big__content">'+(L.busy?'Loading...':'Show more')+'</span></button></div>':'')+(L.source?'<div class="bm-src">Results from '+esc(L.source)+'</div>':'');
  }
  function load(reset){
    if(L.busy)return;L.busy=true;if(reset){L.page=0;L.error='';}
    var it=$('#bm-items');if(it&&reset)it.classList.add('bm-dimmed');
    var qs='q='+encodeURIComponent(L.q)+'&s='+encodeURIComponent(L.s)+'&sort='+encodeURIComponent(L.sort)+'&page='+L.page+'&nsfw='+L.nsfw+(L.m!==''?'&m='+L.m:'');
    return getJSON('/site/beatmaps?'+qs).then(function(d){
      L.items=reset?(d.sets||[]):L.items.concat(d.sets||[]);L.more=!!d.has_more;L.source=d.source||'';
    }).catch(function(e){if(reset)L.items=[];L.more=false;L.error=e.message||'Could not load beatmaps.';}).then(function(){
      L.busy=false;var it2=$('#bm-items');if(it2)it2.classList.remove('bm-dimmed');renderItems();
    });
  }
  function listing(){
    readQuery();document.title='beatmap listing | osu!';
    root.innerHTML=pageHeader('beatmap listing',[['listing','/beatmapsets',true]])+
      (NEXT?'<div class="nxl-top"><div class="nxl-banner"><div class="nxl-banner__bg"></div><h1 class="nxl-banner__title">Beatmaps</h1></div></div>':'')+'<div id="bm-search-area"></div><div id="bm-sort-area"></div>'+footer();
    var sa=$('#bm-search-area'),so=$('#bm-sort-area');
    function drawSearch(){var v=$('#bm-q');var had=v&&document.activeElement===v;sa.innerHTML=searchHtml();if(had)$('#bm-q').focus();}
    function drawSort(){var keep=$('#bm-items')?null:1;if(keep||!$('#bm-items')){so.innerHTML=sortHtml();}else{var t=$('.bm-toolbar',so);var tmp=document.createElement('div');tmp.innerHTML=sortHtml();t.replaceWith($('.bm-toolbar',tmp));}}
    drawSearch();drawSort();
    var timer=null;
    function run(){pushQuery();load(true);}
    sa.addEventListener('input',function(e){if(e.target.id==='bm-q'){L.q=e.target.value.trim();clearTimeout(timer);timer=setTimeout(run,450);}});
    sa.addEventListener('keydown',function(e){if(e.target.id==='bm-q'&&e.key==='Enter'){clearTimeout(timer);L.q=e.target.value.trim();run();}});
    sa.addEventListener('click',function(e){
      var f=e.target.closest('[data-f]');
      if(f){L[f.getAttribute('data-f')]=f.getAttribute('data-v');drawSearch();run();return;}
      if(e.target.closest('#bm-expand')){L.expanded=!L.expanded;drawSearch();}
    });
    so.addEventListener('click',function(e){
      var s=e.target.closest('[data-sort]');
      if(s){e.preventDefault();var k=s.getAttribute('data-sort'),def=SORTS.filter(function(x){return x[0]===k;})[0];
        L.sort=(L.sort===def[2])?def[3]:def[2];drawSort();run();return;}
      var z=e.target.closest('[data-size]');
      if(z){L.size=z.getAttribute('data-size');try{localStorage.setItem('los_bm_size',L.size);}catch(x){}drawSort();renderItems();return;}
      var pv=e.target.closest('[data-preview]');
      if(pv){e.preventDefault();togglePreview(pv,pv.getAttribute('data-preview'));return;}
      if(e.target.closest('#bm-more')){e.preventDefault();L.page++;load(false).then(function(){});}
    });
    so.addEventListener('keydown',function(){});
    load(true);
  }

  var S=null,sel=null;
  function normMods(m){
    m=String(m||'').replace(/^\+/,'').replace(/[^A-Za-z0-9,]/g,'');
    if(!m||m==='NM'||m==='None')return [];
    if(m.indexOf(',')>=0)return m.split(',');
    return m.match(/.{1,2}/g)||[];
  }
  function flag(c){
    c=String(c||'').toUpperCase();if(!/^[A-Z]{2}$/.test(c)||c==='XX')return '';
    var cp=c.split('').map(function(ch){return (0x1f1e6+ch.charCodeAt(0)-65).toString(16);}).join('-');
    return '<img src="'+OSU+'/assets/images/flags/'+cp+'.svg" alt="'+c+'" title="'+c+'" width="20" height="14" style="vertical-align:middle;margin-right:6px">';
  }
  var GRADE={X:'SS.a21de890',XH:'SS-Silver.6681366c',S:'S.3b4498a9',SH:'S-Silver.811ae28c',A:'A.d785e824',B:'B.e19fc91b',C:'C.6bb75adc',D:'D.6b170c4c',F:'F.9a26ed95'};
  function gradeImg(g){var f=GRADE[g]||GRADE.D;return '<img class="gr" src="/site/static/vendor/GradeSmall-'+f+'.svg" alt="'+esc(g)+'">';}
  function pickInitial(set){
    var h=(location.hash||'').replace(/^#/,'').split('/'),maps=set.beatmaps||[];
    if(h[1]){var b=maps.filter(function(x){return String(x.id)===h[1];})[0];if(b)return b;}
    if(h[0]){var mi=MODE_KEY.indexOf(h[0]);if(mi>=0){var inMode=maps.filter(function(x){return (x.mode||0)===mi;});if(inMode.length)return inMode[inMode.length-1];}}
    return maps[0]||null;
  }
  function barRow(label,val,max,hl){var w=Math.max(0,Math.min(100,(Number(val)||0)/max*100));return '<div class="bm-bar'+(hl?' bm-bar--star':'')+'"><span>'+label+'</span><i><b style="width:'+w+'%"></b></i><span class="n">'+fix(val,1)+'</span></div>';}
  function ratingsHtml(r){
    if(!r||!r.length)return '';
    var tot=0,neg=0,pos=0,sum=0,max=1;
    r.forEach(function(v,i){tot+=v;sum+=v*i;if(i<=5)neg+=v;else pos+=v;if(v>max)max=v;});
    if(!tot)return '<div class="beatmapset-stats__row bm-rate"><div class="bm-rate__t">User Rating</div><div class="bm-rate__none">no ratings yet</div></div>';
    var pp=pos/(pos+neg||1)*100;
    return '<div class="beatmapset-stats__row bm-rate"><div class="bm-rate__t">User Rating ('+fix(sum/tot,2)+')</div><div class="bm-rate__bar"><i style="width:'+pp+'%"></i></div><div class="bm-rate__n"><span>'+num(neg)+'</span><span>'+num(pos)+'</span></div>'+
      '<div class="bm-rate__t" style="margin-top:8px">Rating Spread</div><div class="bm-ratings">'+r.slice(0,11).map(function(v){return '<i style="height:'+Math.max(3,v/max*100)+'%" title="'+v+'"></i>';}).join('')+'</div></div>';
  }
  function descHtml(d){
    d=String(d||'');
    if(window.Bbcode)return window.Bbcode.render(d);
    var t=document.createElement('div');t.innerHTML=d;return esc(t.textContent||'');
  }
  function rank(g){return '<div class="score-rank score-rank--tiny score-rank--'+esc(g||'D')+'"></div>';}
  function iconUrl(n){return OSU+'/images/layout/beatmapset-page/'+n+'.svg';}
  function pctOf(v,m){return Math.max(0,Math.min(100,(Number(v)||0)/m*100));}
  function statRow(label,cls,val,max){return '<tr><th class="beatmap-stats-table__label">'+label+'</th><td class="beatmap-stats-table__bar"><div class="bar bar--beatmap-stats bar--beatmap-stats-'+cls+'" style="--fill: '+pctOf(val,max)+'%;"><div class="bar__fill"></div></div></td><td class="beatmap-stats-table__value">'+fix(val,2)+'</td></tr>';}
  function ratingBlock(r){
    var tot=0,neg=0,pos=0,sum=0,max=1;
    (r||[]).forEach(function(v,i){tot+=v;sum+=v*i;if(i<=5)neg+=v;else pos+=v;if(v>max)max=v;});
    if(!tot)return '';
    return '<div class="beatmapset-stats__row beatmapset-stats__row--rating"><div class="beatmapset-stats__rating-header">User Rating ('+fix(sum/tot,2)+')</div>'+
      '<div class="bar bar--beatmap-rating" style="--fill: '+(pos/(pos+neg||1)*100)+'%;"><div class="bar__fill"></div></div>'+
      '<div class="beatmapset-stats__rating-values"><span>'+num(neg)+'</span><span>'+num(pos)+'</span></div>'+
      '<div class="beatmapset-stats__rating-header">Rating Spread</div><div class="beatmapset-stats__rating-chart"><div class="stacked-bar-chart stacked-bar-chart--beatmap-fail-rate">'+
      r.slice(0,11).map(function(v){return '<div class="stacked-bar-chart__col"><div class="stacked-bar-chart__entry" style="height: '+(v/max*100)+'%;"></div></div>';}).join('')+'</div></div></div>';
  }
  var BOARD={type:'global',mods:[]};
  var BOARD_MODS=[['NM','No Mod','System'],['EZ','Easy','DifficultyReduction'],['NF','No Fail','DifficultyReduction'],['HT','Half Time','DifficultyReduction'],['HR','Hard Rock','DifficultyIncrease'],['SD','Sudden Death','DifficultyIncrease'],['PF','Perfect','DifficultyIncrease'],['DT','Double Time','DifficultyIncrease'],['NC','Nightcore','DifficultyIncrease'],['HD','Hidden','DifficultyIncrease'],['FL','Flashlight','DifficultyIncrease'],['SO','Spun Out','Automation'],['TD','Touch Device','System']];
  function showPage(){
    var set=S,b=sel,maps=set.beatmaps||[],modes=[];
    maps.forEach(function(m){if(modes.indexOf(m.mode||0)<0)modes.push(m.mode||0);});
    modes.sort();
    var curMode=b?(b.mode||0):0,inMode=maps.filter(function(m){return (m.mode||0)===curMode;});
    var tags=String(set.tags||'').split(/\s+/).filter(Boolean);
    var modeBar='<ul class="game-mode game-mode--beatmapset">'+MODES.map(function(m){var n=maps.filter(function(x){return (x.mode||0)===m[2];}).length;if(!n)return '';
      return '<li><a class="game-mode-link'+(m[2]===curMode?' game-mode-link--active':'')+'" href="/beatmapsets/'+set.id+'#'+m[0]+'" data-mode="'+m[2]+'"><span class="fal fa-extra-mode-'+m[0]+'" title="'+m[1]+'"></span>'+(n>1||m[2]===curMode?'<span class="game-mode-link__badge">'+n+'</span>':'')+'</a></li>';}).join('')+'</ul>';
    var ranked=['ranked','approved','loved','qualified'].indexOf(set.status)>=0&&set.ranked_date;
    var bgSlim='https://assets.ppy.sh/beatmaps/'+set.id+'/covers/slimcover.jpg';
    var h=pageHeader('beatmap info',[['info','/beatmapsets/'+set.id,true]],modeBar,'beatmapset',bgSlim);
    var nxModes='';
    if(Y19)nxModes='<div class="nxb-modes">'+MODES.map(function(m){var n=maps.filter(function(x){return (x.mode||0)===m[2];}).length;return '<a class="nxb-mode'+(m[2]===curMode?' nxb-mode--active':'')+(n?'':' nxb-mode--empty')+'" href="'+(n?'/beatmapsets/'+set.id+'#'+m[0]:'#')+'" data-mode="'+m[2]+'">'+m[1]+(n?'<span class="nxb-mode__n">'+n+'</span>':'')+'</a>';}).join('')+'</div>';
    if(NEXT)nxModes='<div class="nxb-modes">'+MODES.map(function(m){var n=maps.filter(function(x){return (x.mode||0)===m[2];}).length;return n?'<a class="nxb-mode'+(m[2]===curMode?' nxb-mode--active':'')+'" href="/beatmapsets/'+set.id+'#'+m[0]+'" data-mode="'+m[2]+'">'+m[1]+'<span class="nxb-mode__n">'+n+'</span></a>':'';}).join('')+'</div>';
    var diffBadge=function(st,extra){var c=diffColour(st);return '<div class="difficulty-badge difficulty-badge--beatmapset" style="--bg: '+c+'; color: '+(st>=6.5?'rgb(246,240,92)':'rgb(0,0,0)')+';"><span class="difficulty-badge__icon"><span class="fas fa-star"></span></span><span class="difficulty-badge__rating">'+fix(st,2)+'</span></div>';};
    var q=function(k,v){return '/beatmapsets?q='+encodeURIComponent(k+'="'+v+'"');};
    h+='<div class="osu-layout osu-layout--full"><div class="bm-band bm-band--top"><div class="osu-page osu-page--generic-compact">'+nxModes+'<div class="beatmapset-header">'+
      '<div class="beatmapset-header__cover"><div class="beatmapset-cover beatmapset-cover--full" style="'+cover(set.id,'cover')+'"></div></div>'+
      '<div class="beatmapset-header__box beatmapset-header__box--main"><div class="beatmapset-header__beatmap-picker-box"><div class="beatmapset-beatmap-picker">'+
      inMode.map(function(m){return '<a class="beatmapset-beatmap-picker__beatmap'+(b&&m.id===b.id?' beatmapset-beatmap-picker__beatmap--active':'')+'" href="/beatmapsets/'+set.id+'#'+MODE_KEY[m.mode||0]+'/'+m.id+'" data-map="'+m.id+'" title="'+esc(m.version)+' ('+fix(m.stars,2)+'★)"><div class="beatmap-icon beatmap-icon--beatmapset" style="--diff: '+diffColour(m.stars)+';"><i class="fal fa-extra-mode-'+MODE_KEY[m.mode||0]+'"></i></div></a>';}).join('')+'</div>'+
      (b?'<span class="beatmapset-header__diff-name">'+diffBadge(b.stars)+esc(b.version)+'</span>':'')+
      '<div><span class="beatmapset-header__value" title="Playcount"><span class="beatmapset-header__value-icon"><span class="fas fa-play-circle"></span></span><span class="beatmapset-header__value-name">'+num(set.plays)+'</span></span>'+
      '<span class="beatmapset-header__value beatmapset-header__value--has-favourites"><span class="beatmapset-header__value-icon"><span class="fas fa-heart"></span></span><span class="beatmapset-header__value-name">'+num(set.favourites)+'</span></span></div></div>'+
      '<span class="beatmapset-header__details-text beatmapset-header__details-text--title"><a class="beatmapset-header__details-text-link" href="'+q('title',set.title)+'">'+esc(set.title)+'</a></span>'+
      '<span class="beatmapset-header__details-text beatmapset-header__details-text--artist"><a class="beatmapset-header__details-text-link" href="'+q('artist',set.artist)+'">'+esc(set.artist)+'</a></span>'+
      '<div class="beatmapset-mapping">'+(set.user_id?'<a class="js-usercard" href="/users/'+set.user_id+'"><span class="avatar avatar--guest avatar--beatmapset" style="background-image: url(&quot;https://a.ppy.sh/'+set.user_id+'&quot;);"></span></a>':'')+
      '<div class="beatmapset-mapping__content"><div class="beatmapset-mapping__mapper">mapped by '+(set.user_id?'<a class="js-usercard beatmapset-mapping__user" href="/users/'+set.user_id+'">'+esc(set.creator)+'</a>':'<span class="beatmapset-mapping__user">'+esc(set.creator)+'</span>')+'</div>'+
      (set.submitted_date?'<div>submitted <strong><time>'+esc(fmtDate(set.submitted_date))+'</time></strong></div>':'')+(ranked?'<div>'+(set.status==='loved'?'loved':set.status==='qualified'?'qualified':'ranked')+' <strong><time>'+esc(fmtDate(set.ranked_date))+'</time></strong></div>':(set.last_updated?'<div>last updated <strong><time>'+esc(fmtDate(set.last_updated))+'</time></strong></div>':''))+'</div></div>'+
      '<div class="beatmapset-header__buttons">'+
      '<a class="btn-osu-big btn-osu-big--beatmapset-header" href="/d/'+set.id+'"><span class="btn-osu-big__content"><span class="btn-osu-big__left"><span class="btn-osu-big__text-top">Download</span></span><span class="btn-osu-big__icon"><span class="fa fa-fw"><span class="fas fa-download"></span></span></span></span></a>'+
      (set.video?'<a class="btn-osu-big btn-osu-big--beatmapset-header" href="/d/'+set.id+'n"><span class="btn-osu-big__content"><span class="btn-osu-big__left"><span class="btn-osu-big__text-top">Download</span><span class="btn-osu-big__text-bottom">without Video</span></span><span class="btn-osu-big__icon"><span class="fa fa-fw"><span class="fas fa-download"></span></span></span></span></a>':'')+
      '<a class="btn-osu-big btn-osu-big--beatmapset-header" href="osu://b/'+(b?b.id:set.id)+'"><span class="btn-osu-big__content"><span class="btn-osu-big__left"><span class="btn-osu-big__text-top">osu!direct</span></span><span class="btn-osu-big__icon"><span class="fa fa-fw"><span class="fas fa-download"></span></span></span></span></a></div></div>'+
      '<div class="beatmapset-header__box beatmapset-header__box--stats"><div class="beatmapset-header__status"><span class="beatmapset-status beatmapset-status--show">'+esc(STATUS[set.status]||set.status)+'</span></div>'+
      (b?'<div class="beatmapset-stats"><button type="button" class="beatmapset-stats__row beatmapset-stats__row--preview" data-preview="'+set.id+'"><span class="play-button"></span><div class="beatmapset-stats__elapsed-bar"></div></button>'+
      '<div class="beatmapset-stats__row beatmapset-stats__row--basic"><div class="beatmap-basic-stats">'+
      [['Length (Drain length: '+fmtLen(b.hit_length||b.length)+')','total_length',fmtLen(b.length||b.hit_length)],['BPM','bpm',fix(b.bpm||set.bpm,2)],['Circle Count','count_circles',num(b.circles)],['Slider Count','count_sliders',num(b.sliders)]].map(function(e){return '<div class="beatmap-basic-stats__entry" title="'+esc(e[0])+'"><div class="beatmap-basic-stats__entry-icon" style="background-image: url(&quot;'+iconUrl(e[1])+'&quot;);"></div><span>'+e[2]+'</span></div>';}).join('')+'</div></div>'+
      '<div class="beatmapset-stats__row beatmapset-stats__row--advanced"><table class="beatmap-stats-table"><tbody>'+statRow(curMode===3?'Key Count':'Circle Size','cs',b.cs,10)+statRow('HP Drain','drain',b.hp,10)+statRow('Accuracy','accuracy',b.od,10)+statRow('Approach Rate','ar',b.ar,10)+statRow('Star Rating','stars',b.stars,10)+'</tbody></table></div>'+
      ratingBlock(set.ratings)+'</div>':'')+
      '</div></div>'+
      '<div class="beatmapset-info u-fancy-scrollbar"><div class="beatmapset-info__box"><div class="beatmapset-info__scrollable">'+
      '<div class="beatmapset-info__row"><h3 class="beatmapset-info__header beatmapset-info__header--sticky">Description</h3><div>'+(set.description?descHtml(set.description):'<span style="opacity:.6">No description available from the mirror.</span>')+'</div></div></div></div>'+
      '<div class="beatmapset-info__box"><div class="beatmapset-info__scrollable">'+
      '<div class="beatmapset-info__row beatmapset-info__row--half">'+(set.genre?'<div class="beatmapset-info__half-entry"><h3 class="beatmapset-info__header">Genre</h3><a class="beatmapset-info__link" href="/beatmapsets?q='+encodeURIComponent(set.genre)+'">'+esc(set.genre)+'</a></div>':'')+(set.language?'<div class="beatmapset-info__half-entry"><h3 class="beatmapset-info__header">Language</h3><a class="beatmapset-info__link" href="/beatmapsets?q='+encodeURIComponent(set.language)+'">'+esc(set.language)+'</a></div>':'')+'</div>'+
      (set.source?'<div class="beatmapset-info__row"><h3 class="beatmapset-info__header beatmapset-info__header--sticky">Source</h3><div><a class="beatmapset-info__link" href="/beatmapsets?q='+encodeURIComponent(set.source)+'">'+esc(set.source)+'</a></div></div>':'')+
      (tags.length?'<div class="beatmapset-info__row"><h3 class="beatmapset-info__header beatmapset-info__header--sticky">Mapper Tags</h3><div>'+tags.slice(0,80).map(function(t){return '<a class="beatmapset-info__link" href="/beatmapsets?q='+encodeURIComponent(t)+'">'+esc(t)+'</a>';}).join(' ')+'</div></div>':'')+
      '</div></div>'+
      '<div class="beatmapset-info__box">'+(b&&b.playcount?'<div class="beatmap-success-rate"><h3 class="beatmap-success-rate__header">Success Rate</h3><div class="bar bar--beatmap-success-rate" style="--fill: '+pctOf((b.passcount||0),b.playcount)+'%;"><div class="bar__fill"></div></div><div class="beatmap-success-rate__count-text">'+fix((b.passcount||0)/b.playcount*100,1)+'% ('+num(b.passcount)+' of '+num(b.playcount)+' plays)</div></div>':'')+'</div></div></div></div>'+
      '<div class="bm-band bm-band--board"><div class="osu-page osu-page--generic-compact"><div class="user-profile-pages user-profile-pages--no-tabs"><div class="page-extra"><div class="beatmapset-scoreboard" id="bm-board"></div></div></div></div></div></div>'+footer();
    root.innerHTML=h;
    document.title=set.artist+' - '+set.title+' | osu!';
    loadBoard();
  }
  function modsParam(){return BOARD.mods.length?BOARD.mods.join(','):'';}
  function cell(inner,href,cls){return '<td class="beatmap-scoreboard-table__cell"><a class="beatmap-scoreboard-table__cell-content'+(cls?' '+cls:'')+'" href="'+href+'">'+inner+'</a></td>';}
  function modsHtml(str){var m=modPills(str);return m?'<div class="mods">'+m+'</div>':'';}
  function hitDefs(mode){
    return mode===3?[['max','geki','hit-perfect'],['300','n300','hit-great'],['200','katu','hit-good'],['100','n100','hit-ok'],['50','n50','hit-meh'],['miss','miss','hit-miss']]
      :mode===2?[['fruits','n300','hit-large_tick_hit'],['ticks','n100','hit-small_tick_hit'],['drp miss','katu','hit-small_tick_miss'],['miss','miss','hit-miss']]
      :mode===1?[['great','n300','hit-great'],['ok','n100','hit-ok'],['miss','miss','hit-miss']]
      :[['great','n300','hit-great'],['ok','n100','hit-ok'],['meh','n50','hit-meh'],['miss','miss','hit-miss']];
  }
  function topCard(s,mode,mapKey){
    var href='/scores/'+mapKey+'/'+s.id,acc=s.acc>1.0001?s.acc:s.acc*100;
    var hits=hitDefs(mode).map(function(d){return '<div class="beatmap-score-top__stat"><div class="beatmap-score-top__stat-header beatmap-score-top__stat-header--'+d[2]+'">'+d[0]+'</div><div class="beatmap-score-top__stat-value beatmap-score-top__stat-value--smaller beatmap-score-top__stat-value--'+d[2]+'">'+num(s[d[1]])+'</div></div>';}).join('');
    var fl=flagCountry(s.country);
    return '<div class="beatmap-score-top"><a class="beatmap-score-top__link-container" href="'+href+'"></a><div class="beatmap-score-top__section">'+
      '<div class="beatmap-score-top__wrapping-container beatmap-score-top__wrapping-container--left"><div class="beatmap-score-top__position"><div class="beatmap-score-top__position-number">#'+s.pos+'</div>'+rank(s.grade)+'</div>'+
      '<div class="beatmap-score-top__avatar"><a class="u-hover" href="/users/'+encodeURIComponent(s.player)+'"><span class="avatar avatar--guest" style="background-image: url(&quot;/site/avatar?name='+encodeURIComponent(s.player)+'&quot;);"></span></a></div>'+
      '<div class="beatmap-score-top__user-box"><a class="js-usercard beatmap-score-top__username u-hover" href="/users/'+encodeURIComponent(s.player)+'">'+esc(s.player)+'</a><div class="beatmap-score-top__achieved u-hover">achieved <time>'+esc(ago(s.time))+'</time></div>'+(fl?'<div class="beatmap-score-top__flags">'+fl+'</div>':'')+'</div></div>'+
      '<div class="beatmap-score-top__wrapping-container beatmap-score-top__wrapping-container--right">'+
      '<div class="beatmap-score-top__stats"><div class="beatmap-score-top__stat"><div class="beatmap-score-top__stat-header beatmap-score-top__stat-header--wider">Total Score</div><div class="beatmap-score-top__stat-value beatmap-score-top__stat-value--score">'+num(s.score)+'</div></div></div>'+
      '<div class="beatmap-score-top__stats"><div class="beatmap-score-top__stat"><div class="beatmap-score-top__stat-header beatmap-score-top__stat-header--wider">Accuracy</div><div class="beatmap-score-top__stat-value">'+acc.toFixed(2)+'%</div></div><div class="beatmap-score-top__stat"><div class="beatmap-score-top__stat-header beatmap-score-top__stat-header--wider">Max Combo</div><div class="beatmap-score-top__stat-value">'+num(s.combo)+'x</div></div></div>'+
      '<div class="beatmap-score-top__stats beatmap-score-top__stats--wrappable">'+hits+
      '<div class="beatmap-score-top__stat"><div class="beatmap-score-top__stat-header">pp</div><div class="beatmap-score-top__stat-value beatmap-score-top__stat-value--smaller u-hover">'+(s.pp>0?'<span class="pp-value" title="'+s.pp+'">'+num(Math.round(s.pp))+'</span>':'-')+'</div></div>'+
      '<div class="beatmap-score-top__stat"><div class="beatmap-score-top__stat-header">Time</div><div class="beatmap-score-top__stat-value beatmap-score-top__stat-value--smaller u-hover"><time>'+esc(ago(s.time))+'</time></div></div>'+
      '<div class="beatmap-score-top__stat"><div class="beatmap-score-top__stat-header beatmap-score-top__stat-header--mods">Mods</div><div class="beatmap-score-top__stat-value beatmap-score-top__stat-value--mods u-hover">'+(modsHtml(s.mods)||'-')+'</div></div>'+
      '</div></div></div></div>';
  }
  function boardRow(s,mode,mapKey){
    var href='/scores/'+mapKey+'/'+s.id,acc=s.acc>1.0001?s.acc:s.acc*100;
    var cls='beatmap-scoreboard-table__body-row beatmap-scoreboard-table__body-row--highlightable'+(s.pos===1?' beatmap-scoreboard-table__body-row--first':'')+(s.friend&&!s.self?' beatmap-scoreboard-table__body-row--friend':'')+(s.self?' beatmap-scoreboard-table__body-row--self':'');
    var hits=hitDefs(mode).map(function(d){var v=Number(s[d[1]])||0;return cell(num(v),href,'beatmap-scoreboard-table__cell-content--'+d[2]+(v===0?' beatmap-scoreboard-table__cell-content--zero':''));}).join('');
    return '<tr class="'+cls+'">'+cell('#'+s.pos,href,'beatmap-scoreboard-table__cell-content--rank')+cell(rank(s.grade),href,'beatmap-scoreboard-table__cell-content--grade')+cell(num(s.score),href,'beatmap-scoreboard-table__cell-content--score')+cell(acc.toFixed(2)+'%',href)+
      '<td class="beatmap-scoreboard-table__cell">'+(s.country?'<a class="beatmap-scoreboard-table__cell-content" href="'+href+'">'+flagCountry(s.country)+'</a>':'')+'</td>'+
      '<td class="beatmap-scoreboard-table__cell beatmap-scoreboard-table__cell--player u-relative"><a class="beatmap-scoreboard-table__cell-content beatmap-scoreboard-table__cell-content--bg-link" href="'+href+'"></a><span class="beatmap-scoreboard-table__cell-content u-hover-none"><a class="js-usercard beatmap-scoreboard-table__user-link u-hover" href="/users/'+encodeURIComponent(s.player)+'">'+esc(s.player)+'</a></span></td>'+
      cell(num(s.combo)+'x',href)+hits+cell(s.pp>0?'<span class="pp-value" title="'+s.pp+'">'+num(Math.round(s.pp))+'</span>':'-',href)+cell('<time title="'+esc(fmtFull(s.time))+'">'+esc(agoShort(s.time))+'</time>',href,'beatmap-scoreboard-table__cell-content--time')+
      '<td class="beatmap-scoreboard-table__cell beatmap-scoreboard-table__cell--player u-relative"><a class="beatmap-scoreboard-table__cell-content beatmap-scoreboard-table__cell-content--bg-link" href="'+href+'"></a><div class="beatmap-scoreboard-table__mods">'+modsHtml(s.mods)+'</div></td></tr>';
  }
  function agoShort(ts){
    var t=Number(ts)||0;if(t>1e12)t/=1000;if(!t)return '';
    var d=Math.max(0,Math.round(Date.now()/1000-t)),u=[['y',31536000],['mo',2592000],['d',86400],['h',3600],['m',60]];
    for(var i=0;i<u.length;i++)if(d>=u[i][1])return Math.floor(d/u[i][1])+u[i][0];
    return 'now';
  }
  function nextRow(s,mode,mapKey,lead){
    var href='/scores/'+mapKey+'/'+s.id,acc=s.acc>1.0001?s.acc:s.acc*100;
    var fl=flagCountry(s.country),hits=mode===3?[s.geki,s.n300,s.katu,s.n100,s.n50,s.miss].map(num).join('/'):(mode===1?[s.n300,s.n100,s.miss]:[s.n300,s.n100,s.n50]).map(num).join('/');
    if(lead)return '<div class="nxb-lead"><div class="nxb-lead__title">In the Lead</div><a class="nxb-lead__card" href="'+href+'"><div class="nxb-lead__head"><span class="nxb-av"><img src="/site/avatar?name='+encodeURIComponent(s.player)+'" alt="" onerror="this.style.visibility=\'hidden\'"></span><div class="nxb-lead__who"><span class="nxb-name">'+esc(s.player)+'</span>'+fl+'</div><div class="nxb-lead__pos"><b>#'+s.pos+'</b><small>achieved '+esc(ago(s.time))+'</small></div></div>'+
      '<div class="nxb-lead__body"><div class="nxb-grade">'+rank(s.grade)+'</div><div class="nxb-stat"><small>Score</small><b>'+num(s.score)+'</b></div><div class="nxb-stat"><small>Accuracy</small><b>'+acc.toFixed(1)+'%</b></div><div class="nxb-stat"><small>'+(mode===3?'MAX/300/200/100/50/Miss':mode===1?'300/100/Miss':'300/100/50')+'</small><b>'+hits+'</b></div><div class="nxb-mods">'+modsHtml(s.mods)+'</div></div></a></div>';
    return '<a class="nxb-row'+(s.self?' nxb-row--self':'')+(s.friend?' nxb-row--friend':'')+'" href="'+href+'"><span class="nxb-rank">#'+s.pos+'</span><span class="nxb-flag">'+fl+'</span><span class="nxb-name">'+esc(s.player)+'</span><span class="nxb-mods">'+modsHtml(s.mods)+'</span><span class="nxb-grade">'+rank(s.grade)+'</span><span class="nxb-score">'+num(s.score)+'</span><span class="nxb-acc">'+acc.toFixed(2)+'%</span><span class="nxb-hits">'+hits+'</span></a>';
  }
  function loadBoard(){
    var el=$('#bm-board');if(!el||!sel)return;
    var b=sel,mode=b.mode||0,mapKey=MODE_KEY[mode];
    var tabs=[['global','Global Ranking'],['country','Country Ranking'],['friend','Friend Ranking']];
    var head='<div class="page-tabs">'+tabs.map(function(t){return '<div class="page-tabs__tab'+(BOARD.type===t[0]?' page-tabs__tab--active':'')+'" data-bt="'+t[0]+'"><span class="fake-bold" data-content="'+t[1]+'">'+t[1]+'</span></div>';}).join('')+'</div>'+
      '<div class="beatmapset-scoreboard__mods'+(BOARD.mods.length?'':' beatmapset-scoreboard__mods--initial')+'">'+BOARD_MODS.map(function(m){return '<button type="button" class="beatmap-scoreboard-mod'+(BOARD.mods.indexOf(m[0])>=0?' beatmap-scoreboard-mod--enabled':'')+'" data-bmod="'+m[0]+'"><div class="mod mod--type-'+m[2]+'" title="'+m[1]+'"><div class="mod__icon mod__icon--'+m[0]+'" data-acronym="'+m[0]+'"></div></div></button>';}).join('')+'</div>';
    el.innerHTML=head+'<div class="beatmapset-scoreboard__main beatmapset-scoreboard__main--loading"><div class="beatmapset-scoreboard__notice">Loading...</div></div>';
    bindBoard(el);
    var url='/site/beatmap_scores?id='+b.id+'&md5='+encodeURIComponent(b.checksum||'')+'&m='+mode+'&type='+BOARD.type+(modsParam()?'&mods='+encodeURIComponent(modsParam()):'');
    getJSON(url).then(function(d){
      if(sel!==b)return;
      var sc=d.scores||[],body;
      if(d.requires_login)body='<div class="beatmapset-scoreboard__notice">Please sign in to see this ranking.</div>';
      else if(BOARD.type==='country'&&!d.viewer_country)body='<div class="beatmapset-scoreboard__notice">Your account has no country set, so there is no country ranking to show.</div>';
      else if(!sc.length)body='<div class="beatmapset-scoreboard__notice">No scores yet. Maybe you should set a new one?</div>';
      else{
        if(NEXT){var m0=$('.beatmapset-scoreboard__main',el);if(m0){m0.className='beatmapset-scoreboard__main';m0.innerHTML='<div class="nxb-board">'+nextRow(sc[0],mode,mapKey,true)+(d.me&&d.me.pos>1?'<div class="nxb-sep">Your best</div>'+nextRow(d.me,mode,mapKey,false):'')+'<div class="nxb-rows">'+sc.slice(1).map(function(r){return nextRow(r,mode,mapKey,false);}).join('')+'</div></div>';}return;}
        var top='<div class="beatmap-scoreboard-top">'+topCard(sc[0],mode,mapKey)+(d.me&&d.me.pos>1?topCard(d.me,mode,mapKey):'')+'</div>';
        var hd=hitDefs(mode).map(function(x){return '<th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--hitstat beatmap-scoreboard-table__header--'+x[2]+'">'+x[0]+'</th>';}).join('');
        body='<div>'+top+'<div class="beatmap-scoreboard-table"><table class="beatmap-scoreboard-table__table"><thead><tr><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--rank">Rank</th><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--grade"></th><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--score">Score</th><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--accuracy">Accuracy</th><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--flag"></th><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--player">Player</th><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--maxcombo">Max Combo</th>'+hd+'<th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--pp">pp</th><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--time">Time</th><th class="beatmap-scoreboard-table__header beatmap-scoreboard-table__header--mods">Mods</th></tr></thead><tbody class="beatmap-scoreboard-table__body">'+
          sc.map(function(s){return boardRow(s,mode,mapKey);}).join('')+'</tbody></table></div></div>';
      }
      var m=$('.beatmapset-scoreboard__main',el);if(m){m.className='beatmapset-scoreboard__main';m.innerHTML=body;}
    }).catch(function(){var m=$('.beatmapset-scoreboard__main',el);if(m){m.className='beatmapset-scoreboard__main';m.innerHTML='<div class="beatmapset-scoreboard__notice">Could not load scores.</div>';}});
  }
  function bindBoard(el){
    el.onclick=function(e){
      var t=e.target.closest('[data-bt]');if(t){BOARD.type=t.getAttribute('data-bt');loadBoard();return;}
      var m=e.target.closest('[data-bmod]');if(m){
        var k=m.getAttribute('data-bmod'),i=BOARD.mods.indexOf(k);
        if(k==='NM')BOARD.mods=i>=0?[]:['NM'];
        else{BOARD.mods=BOARD.mods.filter(function(x){return x!=='NM';});i=BOARD.mods.indexOf(k);if(i>=0)BOARD.mods.splice(i,1);else BOARD.mods.push(k);}
        loadBoard();
      }
    };
  }
  function setPage(id,bid){
    root.innerHTML=pageHeader('beatmap info',[['info',id?'/beatmapsets/'+id:'#',true]],'','beatmapset')+'<div class="bm-msg">Loading...</div>'+footer();
    getJSON('/site/beatmapset?'+(id?'id='+id:'bid='+bid)).then(function(d){
      S=d.set;if(!S)throw new Error('Beatmapset not found.');
      sel=null;
      if(bid){sel=(S.beatmaps||[]).filter(function(x){return String(x.id)===String(bid);})[0]||null;if(sel)try{history.replaceState(null,'','/beatmapsets/'+S.id+'#'+MODE_KEY[sel.mode||0]+'/'+sel.id);}catch(e){}}
      if(!sel)sel=pickInitial(S);
      showPage();
      root.addEventListener('click',function(e){
        var pv=e.target.closest('[data-preview]');if(pv){togglePreview(pv,pv.getAttribute('data-preview'));return;}
        var m=e.target.closest('a[data-mode]');
        if(m){e.preventDefault();var mm=Number(m.getAttribute('data-mode')),list=S.beatmaps.filter(function(x){return (x.mode||0)===mm;});sel=list[list.length-1]||sel;sync();showPage();return;}
        var p=e.target.closest('a[data-map]');
        if(p){e.preventDefault();var pid=p.getAttribute('data-map');sel=S.beatmaps.filter(function(x){return String(x.id)===pid;})[0]||sel;sync();showPage();}
      });
    }).catch(function(e){
      root.innerHTML=pageHeader('beatmap info',[['info','#',true]],'','beatmapset')+'<div class="bm-msg">'+esc(e.message||'Could not load this beatmapset.')+'<br><br><a href="/beatmapsets">Back to listing</a></div>'+footer();
    });
  }
  function sync(){if(sel)try{history.replaceState(null,'','/beatmapsets/'+S.id+'#'+MODE_KEY[sel.mode||0]+'/'+sel.id);}catch(e){}}

  var MOD_NAMES={NF:'No Fail',EZ:'Easy',TD:'Touch Device',HD:'Hidden',HR:'Hard Rock',SD:'Sudden Death',DT:'Double Time',RX:'Relax',HT:'Half Time',NC:'Nightcore',FL:'Flashlight',SO:'Spun Out',AP:'Autopilot',PF:'Perfect',V2:'Score V2',CL:'Classic',AT:'Autoplay',CN:'Cinema',FI:'Fade In',MR:'Mirror',RD:'Random',K1:'1K',K2:'2K',K3:'3K',K4:'4K',K5:'5K',K6:'6K',K7:'7K',K8:'8K',K9:'9K',KC:'Key Coop'};
  function modPills(str){
    var mods=normMods(str);mods=mods.filter(function(m,i){return mods.indexOf(m)===i&&!(m==='DT'&&mods.indexOf('NC')>=0)&&!(m==='SD'&&mods.indexOf('PF')>=0);});
    return mods.map(function(m){var t=['EZ','HT','NF'].indexOf(m)>=0?'DifficultyReduction':['RX','AP','AT'].indexOf(m)>=0?'Automation':m==='CL'?'Conversion':'DifficultyIncrease';
      return '<div class="mod mod--type-'+t+'" title="'+esc(MOD_NAMES[m]||m)+'"><div class="mod__icon mod__icon--'+esc(m)+'" data-acronym="'+esc(m)+'"></div></div>';}).join('');
  }
  function fmtFull(ts){var t=Number(ts)||0;if(t<1e12)t*=1000;var d=new Date(t);if(isNaN(d))return '';return d.getUTCDate()+' '+['January','February','March','April','May','June','July','August','September','October','November','December'][d.getUTCMonth()]+' '+d.getUTCFullYear()+' '+('0'+d.getUTCHours()).slice(-2)+':'+('0'+d.getUTCMinutes()).slice(-2);}
  function flagCountry(c){
    c=String(c||'').toUpperCase();if(!/^[A-Z]{2}$/.test(c)||c==='XX')return '';
    var cp=c.split('').map(function(ch){return (0x1f1e6+ch.charCodeAt(0)-65).toString(16);}).join('-');
    return '<span class="flag-country" title="'+c+'" style="background-image:url(&quot;'+OSU+'/assets/images/flags/'+cp+'.svg&quot;)"></span>';
  }
  function statCell(label,val,cls){return '<div class="score-stats__stat"><div class="score-stats__stat-row score-stats__stat-row--label'+(cls&&cls.indexOf('hit-')===0?' score-stats__stat-row--'+cls:'')+'">'+label+'</div><div class="score-stats__stat-row'+(cls&&cls.indexOf('hit-')!==0?' score-stats__stat-row--'+cls:'')+'">'+val+'</div></div>';}
  function scorePage(id){
    root.innerHTML=pageHeader('performance',[],'','scores')+'<div class="bm-msg">Loading...</div>'+footer();
    getJSON('/site/score?id='+id).then(function(sc){
      var bm=sc.beatmap||{},m=sc.mode||0,acc=sc.acc>1.0001?sc.acc:sc.acc*100;
      var mapHref=bm.beatmapset_id>0?'/beatmapsets/'+bm.beatmapset_id+'#'+MODE_KEY[m]+'/'+bm.beatmap_id:'';
      var link=function(inner,cls){return mapHref?'<a class="'+cls+'" href="'+mapHref+'">'+inner+'</a>':'<span class="'+cls+'">'+inner+'</span>';};
      var g=sc.grade;
      var order=['X','S','A','B','C','D'],cur=g==='XH'?0:g==='SH'?1:Math.max(0,order.indexOf(g)<0?5:order.indexOf(g));
      var tower=order.map(function(r,i){
        var shown=i===0?(g==='XH'?'XH':'X'):i===1?(g==='SH'?'SH':'S'):r;
        var cls=i<cur?' score-tower__item--missed':i>cur?' score-tower__item--passed':'';
        return '<div class="score-tower__item'+cls+'"><div class="score-rank score-rank--'+shown+'"></div></div>';}).join('');
      var dial=g==='F'?'D':g;
      var hits;
      if(m===3)hits=statCell('max',num(sc.geki),'hit-perfect')+statCell('300',num(sc.n300),'hit-great')+statCell('200',num(sc.katu),'hit-good')+statCell('100',num(sc.n100),'hit-ok')+statCell('50',num(sc.n50),'hit-meh')+statCell('miss',num(sc.miss),'hit-miss');
      else if(m===2)hits=statCell('fruits',num(sc.n300),'hit-large_tick_hit')+statCell('ticks',num(sc.n100),'hit-small_tick_hit')+statCell('drp miss',num(sc.katu),'hit-small_tick_miss')+statCell('miss',num(sc.miss),'hit-miss');
      else if(m===1)hits=statCell('great',num(sc.n300),'hit-great')+statCell('ok',num(sc.n100),'hit-ok')+statCell('miss',num(sc.miss),'hit-miss');
      else hits=statCell('great',num(sc.n300),'hit-great')+statCell('ok',num(sc.n100),'hit-ok')+statCell('meh',num(sc.n50),'hit-meh')+statCell('miss',num(sc.miss),'hit-miss');
      var mods=modPills(sc.mods);
      var h=pageHeader('performance',[],'','scores');
      h+='<div class="osu-layout__section osu-layout__section--full"><div class="osu-page osu-page--generic-compact">'+
        '<div class="score-beatmap"><h1 class="score-beatmap__title">'+link(esc(bm.title)+' <span class="score-beatmap__artist">by '+esc(bm.artist)+'</span>','score-beatmap__link-plain')+'</h1>'+
        '<div class="score-beatmap__detail"><span class="u-ellipsis-overflow"><div class="beatmap-list-item beatmap-list-item--inline beatmap-list-item--score">'+
        '<div class="beatmap-list-item__col beatmap-list-item__col--icon"><span class="fal fa-extra-mode-'+MODE_KEY[m]+'"></span></div>'+
        '<div class="beatmap-list-item__col"><div class="difficulty-badge" id="sc-badge" style="display:none"><span class="difficulty-badge__icon"><span class="fas fa-star"></span></span><span class="difficulty-badge__rating" id="sc-stars"></span></div></div>'+
        '<div class="beatmap-list-item__col beatmap-list-item__col--main"><div class="beatmap-list-item__version">'+link(esc(bm.version),'beatmap-list-item__version-link')+(bm.creator?' <span class="beatmap-list-item__mapper">mapped by <a href="/beatmapsets?q='+encodeURIComponent(bm.creator)+'">'+esc(bm.creator)+'</a></span>':'')+'</div></div>'+
        '</div></span></div></div>'+
        '<div class="score-info"><div class="score-info__cover"><div class="beatmapset-cover beatmapset-cover--full" style="'+(bm.beatmapset_id>0?cover(bm.beatmapset_id,'cover'):'')+'"></div></div>'+
        '<div class="score-info__item"><div class="score-tower">'+tower+'</div></div>'+
        '<div class="score-info__item score-info__item--dial"><div class="legacy-rank legacy-rank--'+dial+'"></div></div>'+
        '<div class="score-info__item score-info__item--player"><div class="score-player"><div class="score-player__row score-player__row--score">'+(mods?'<div class="score-player__mods">'+mods+'</div>':'')+'<div class="score-player__score">'+num(sc.score)+'</div></div>'+
        '<div class="score-player__row score-player__row--player"><span>Played by</span><strong><a href="/users/'+encodeURIComponent(sc.player)+'" style="color:inherit">'+esc(sc.player)+'</a></strong><span>Submitted on</span><strong>'+esc(fmtFull(sc.time))+'</strong><span>Played on</span><strong>Stable</strong></div>'+
        (sc.rank?'<div class="score-player__row score-player__row--rank"><div class="score-player__rank score-player__rank--label">Global Rank</div><div class="score-player__rank score-player__rank--value"><span>#'+sc.rank+'</span></div></div>':'')+'</div></div>'+
        '<div class="score-info__item score-info__item--buttons"><div class="score-buttons">'+(mapHref?'<a class="btn-osu-big btn-osu-big--rounded" href="'+mapHref+'">Beatmap</a>':'')+'</div></div></div>'+
        '<div class="score-stats"><div class="score-stats__group score-stats__group--user-card"><div class="user-card user-card--card user-card--highlightable">'+
        '<a class="user-card__background-container" href="/users/'+encodeURIComponent(sc.player)+'"><img class="user-card__background" src="/site/cover?name='+encodeURIComponent(sc.player)+'" alt="" onerror="this.style.display=\'none\'"><div class="user-card__background-overlay"></div></a>'+
        '<div class="user-card__card"><div class="user-card__content user-card__content--details"><div class="user-card__user"><div class="user-card__avatar-space"><img class="user-card__avatar user-card__avatar--loaded" src="/site/avatar?name='+encodeURIComponent(sc.player)+'" alt="" onerror="this.onerror=null;this.src=\'/site/static/vendor/avatar-guest@2x.01495bc4.png\'"></div></div>'+
        '<div class="user-card__details"><div class="user-card__icons user-card__icons--card">'+(sc.country?'<span class="user-card__icon user-card__icon--flag">'+flagCountry(sc.country)+'</span>':'')+'</div>'+
        '<div class="user-card__username-row"><a class="user-card__username u-ellipsis-pre-overflow" href="/users/'+encodeURIComponent(sc.player)+'">'+esc(sc.player)+'</a></div></div></div>'+
        '<div class="user-card__content user-card__content--status"><div class="user-card__status"><div class="user-card__status-icon-container"><div class="user-card__status-icon user-card__status-icon--offline"></div></div><div class="user-card__status-messages"><span class="user-card__status-message user-card__status-message--sub u-ellipsis-overflow">Played '+esc(ago(sc.time))+'</span><span class="user-card__status-message u-ellipsis-overflow">Offline</span></div></div></div></div></div></div>'+
        '<div class="score-stats__group score-stats__group--stats"><div class="score-stats__group-row">'+statCell('Accuracy',acc.toFixed(2)+'%')+statCell('Max Combo',num(sc.combo)+'x',sc.perfect?'perfect':'')+statCell('pp',sc.pp>0?'<span class="pp-value" title="'+sc.pp+'">'+num(Math.round(sc.pp))+'</span>':'-')+'</div>'+
        '<div class="score-stats__group-row">'+hits+'</div></div></div>'+
        '</div></div>'+footer();
      root.innerHTML=h;
      document.title=sc.player+' on '+bm.title+' ['+bm.version+'] · performance | osu!';
      if(bm.beatmapset_id>0){
        getJSON('/site/beatmapset?id='+bm.beatmapset_id).then(function(d){
          var set=d.set,b=set&&(set.beatmaps||[]).filter(function(x){return Number(x.id)===Number(bm.beatmap_id);})[0];
          var el=$('#sc-badge');if(b&&el&&b.stars){el.style.cssText='--bg:'+diffColour(b.stars)+';color:'+(b.stars>=6.5?'#f6f05c':'#000');$('#sc-stars').textContent=fix(b.stars,2);}
        }).catch(function(){});
      }
    }).catch(function(e){
      root.innerHTML=pageHeader('performance',[],'','scores')+'<div class="bm-msg">'+esc(e.message||'Could not load this score.')+'<br><br><a href="/">Back to home</a></div>'+footer();
    });
  }

  function boot(){
    var m=location.pathname.replace(/\/+$/,'').match(/\/beatmapsets\/(\d+)$/);
    if(!m){var m2=location.pathname.match(/\/beatmapsets\/(\d+)/);m=m2;}
    var sm=location.pathname.replace(/\/+$/,'').match(/\/scores\/(?:[a-z]+\/)?(\d+)$/);
    var bm=location.pathname.replace(/\/+$/,'').match(/^(?:\/osu)?\/beatmaps\/(\d+)$/);
    var run=function(){if(sm)scorePage(sm[1]);else if(m)setPage(m[1]);else if(bm)setPage(0,bm[1]);else listing();};
    getJSON('/site/session').then(function(s){session.user=s&&s.user||null;session.s=s||{};}).catch(function(){}).then(run);
  }
  boot();
})();
