(function(){
  var $=function(id){return document.getElementById(id);};
  function esc(s){return String(s==null?'':s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];});}
  function num(n){try{return Number(n||0).toLocaleString();}catch(e){return String(n||0);}}
  function url(u){return /^https?:\/\//i.test(u||'')?u:'';}
  var MONTHS=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
  function ago(iso,future){
    var t=Date.parse(iso);if(isNaN(t))return '';
    var d=Math.round((future?t-Date.now():Date.now()-t)/1000);if(d<0)d=0;
    var u=[['year',31536000],['month',2592000],['day',86400],['hour',3600],['minute',60]];
    for(var i=0;i<u.length;i++){if(d>=u[i][1]){var n=Math.floor(d/u[i][1]);return future?'in '+n+' '+u[i][0]+(n>1?'s':''):n+' '+u[i][0]+(n>1?'s':'')+' ago';}}
    return future?'soon':'just now';
  }
  function getJSON(part,extra){
    return fetch('/site/home_feed?part='+part+(extra||''),{credentials:'same-origin'}).then(function(r){
      if(!r.ok)throw new Error(String(r.status));return r.json();
    });
  }
  var OSU='https://osu.ppy.sh';
  var MODES=[['osu','osu!'],['taiko','osu!taiko'],['fruits','osu!catch'],['mania','osu!mania']];

  function header(s){
    var nm=encodeURIComponent(s.user);
    var links=[['home','/'],['beatmaps','/beatmapsets'],['rankings',OSU+'/rankings/osu/performance'],['community',OSU+'/community/forums'],['store',OSU+'/home/support'],['help',OSU+'/wiki/en/Help_centre']];
    var ext=function(h){return h.charAt(0)==='/'?'':' target="_blank" rel="noreferrer"';};
    var nav='<div class="dash-top"><nav class="dash-nav" aria-label="Main navigation">'+
      '<a class="dash-logo nav2__logo-link" href="/" aria-label="osu! home"><div class="nav2__logo nav2__logo--bg"></div><div class="nav2__logo"></div></a>'+
      '<div class="dash-links">'+links.map(function(l){return '<a href="'+l[1]+'"'+ext(l[1])+'>'+l[0]+'</a>';}).join('')+
      '<a href="/beatmapsets" aria-label="search" class="dash-icon"><i class="fas fa-search"></i></a></div>'+
      '<div class="dash-right"><a class="dash-icon" href="https://twitter.com/osugame" target="_blank" rel="noreferrer" aria-label="Twitter"><i class="fab fa-twitter"></i></a>'+
      '<a class="dash-icon" href="'+OSU+'/home/support" target="_blank" rel="noreferrer" aria-label="Support"><i class="fas fa-heart"></i></a>'+
      (window.SiteNav?SiteNav.extras(s):'')+'<div class="dash-user"><button type="button" class="dash-avatar" id="dash-user-btn" aria-haspopup="true" aria-expanded="false" title="'+esc(s.user)+'"><img src="/site/avatar?name='+nm+'" alt="" onerror="this.onerror=null;this.src=\'/site/static/vendor/avatar-guest@2x.01495bc4.png\'"></button>'+
      '<div class="dash-menu" id="dash-user-menu" hidden><div class="dash-menu__name">'+esc(s.user)+'</div>'+
      '<a href="/users/'+nm+'">my profile</a><a href="/home/friends">friends</a><a href="/profile#settings">account settings</a><button type="button" id="dash-switch" data-open-login>switch profile</button><button type="button" id="dash-logout">sign out</button></div></div></div>'+
      '</nav></div>';
    var tabs=[['dashboard','/',true],['friends','/home/friends',false],['watchlists',OSU+'/home/follows',false],['account settings','/profile#settings',false]];
    return '<div class="header-v4 header-v4--home">'+nav+'<div class="header-v4__container header-v4__container--main">'+
      '<div class="header-v4__bg-container"><div class="header-v4__bg u-current-user-cover"></div></div>'+
      '<div class="header-v4__content"><div class="header-v4__row header-v4__row--title"><div class="header-v4__icon"></div><div class="header-v4__title">dashboard</div></div></div></div>'+
      '<div class="header-v4__container"><div class="header-v4__content"><div class="header-v4__row header-v4__row--bar"><ul class="header-nav-v4 header-nav-v4--list">'+
      tabs.map(function(t){return '<li class="header-nav-v4__item"><a class="header-nav-v4__link'+(t[2]?' header-nav-v4__link--active':'')+'" href="'+t[1]+'"'+ext(t[1])+'><span class="fake-bold" data-content="'+t[0]+'">'+t[0]+'</span></a></li>';}).join('')+
      '</ul></div></div></div></div>';
  }
  function footer(){
    return '<div class="dash-footer"><div class="dash-footer__links"><a href="'+OSU+'/wiki/en/Rules" target="_blank" rel="noreferrer">Rules</a><a href="'+OSU+'/legal/terms" target="_blank" rel="noreferrer">Terms</a><a href="'+OSU+'/legal/privacy" target="_blank" rel="noreferrer">Privacy</a><a href="'+OSU+'/legal/copyright" target="_blank" rel="noreferrer">Copyright (DMCA)</a><a href="https://status.ppy.sh" target="_blank" rel="noreferrer">Server Status</a><a href="https://github.com/ppy" target="_blank" rel="noreferrer">Source Code</a></div><div class="dash-footer__powered">ppy powered 2007-2026</div></div>';
  }

  function newsItem(p,collapsed){
    var d=new Date(p.date),ok=!isNaN(d.getTime());
    var img=url(p.image),href=OSU+'/home/news/'+encodeURIComponent(p.slug);
    var my=ok?(collapsed?' '+MONTHS[d.getUTCMonth()]:d.getUTCFullYear()+' '+MONTHS[d.getUTCMonth()]):'';
    return '<div class="news-post-preview'+(collapsed?' news-post-preview--collapsed':'')+'">'+
      '<a class="news-post-preview__image" target="_blank" rel="noreferrer" href="'+href+'"'+(img?' style="--bg:url(\''+esc(img)+'\');--bg-2x:url(\''+esc(img)+'\')"':'')+'></a>'+
      '<div class="news-post-preview__body"><div class="news-post-preview__post-date"><div class="news-post-preview__date">'+(ok?('0'+d.getUTCDate()).slice(-2):'')+'</div><div class="news-post-preview__month-year">'+my+'</div></div>'+
      '<div class="news-post-preview__post-right"><a class="news-post-preview__post-title" target="_blank" rel="noreferrer" href="'+href+'">'+esc(p.title)+'</a>'+
      '<div class="news-post-preview__post-content"><p>'+esc(p.preview)+'</p></div></div></div></div>';
  }
  function newsHtml(d){
    var posts=(d&&d.posts)||[];
    if(!posts.length)return '<div class="dash-note">No news to show right now.</div>';
    var big=posts.slice(0,4),small=posts.slice(4);
    return big.map(function(p){return newsItem(p,false);}).join('')+
      (small.length?'<div class="user-home__news-posts-group">'+small.map(function(p){return newsItem(p,true);}).join('')+'</div>':'')+
      '<a class="user-home__news-posts-group user-home__news-posts-group--more" target="_blank" rel="noreferrer" href="'+OSU+'/home/news">see more</a>';
  }

  function modeIcons(modes){
    return (modes||[]).map(function(m){return '<div class="user-home-beatmapset__playmode-icon" title="'+esc(m)+'"><span class="fal fa-extra-mode-'+esc(m)+'"></span></div>';}).join('');
  }
  function setHtml(b,type,href){
    var c=url(b.cover),c2=url(b.cover2x)||c;
    var right='';
    if(type==='new')right=ago(b.ranked);
    else if(type==='popular')right='<span class="fa fa-heart"></span> '+num(b.favourites);
    else if(type==='daily')right='resets '+ago(b.ends,true);
    return '<a class="user-home-beatmapset" target="_blank" rel="noreferrer" href="'+(href||'/beatmapsets/'+b.id)+'">'+
      '<div class="beatmapset-cover beatmapset-cover--home"'+(c?' style="--bg:url(\''+esc(c)+'\');--bg-2x:url(\''+esc(c2)+'\')"':'')+'></div>'+
      '<div class="user-home-beatmapset__meta"><div class="user-home-beatmapset__title-container">'+(type==='popular'?'':modeIcons(b.modes))+'<div class="user-home-beatmapset__title u-ellipsis-overflow">'+esc(b.title)+'</div></div>'+
      '<div class="user-home-beatmapset__artist u-ellipsis-overflow">'+esc(b.artist)+'</div>'+
      '<div class="user-home-beatmapset__creator u-ellipsis-overflow"><span>by '+esc(b.creator)+'</span><span class="user-home-beatmapset__playcount">'+right+'</span></div></div>'+
      '<div class="user-home-beatmapset__chevron"><i class="fas fa-chevron-right"></i></div></a>';
  }
  function listHtml(d,type,moreHref){
    var sets=(d&&d.sets)||[];
    if(!sets.length)return '<div class="dash-note dash-note--side">Nothing to show right now.</div>';
    return sets.map(function(b){return setHtml(b,type);}).join('')+'<a target="_blank" rel="noreferrer" href="'+moreHref+'">see more</a>';
  }

  function banner(d){
    var imgs=((d&&d.images)||[]).filter(function(i){return url(i.image);});
    if(!imgs.length)return '';
    var one=imgs.length<2;
    return '<div class="menu-images" id="dash-banner"><div class="menu-images__blur" style="background-image:url(\''+esc(imgs[0].image)+'\')"></div>'+
      '<div class="menu-images__images">'+(one?'':'<button class="menu-images__arrow menu-images__arrow--left" type="button" aria-label="previous"><span class="fas fa-chevron-left"></span></button><button class="menu-images__arrow menu-images__arrow--right" type="button" aria-label="next"><span class="fas fa-chevron-right"></span></button>')+
      '<div class="menu-images__container">'+imgs.map(function(i,n){
        var link=url(i.url);
        var im='<img class="menu-image__image" alt="'+esc(i.name||'')+'" src="'+esc(i.image)+'">';
        return '<div class="menu-image" data-n="'+n+'" style="'+(n?'display:none':'')+'">'+(link?'<a target="_blank" rel="noreferrer" href="'+esc(link)+'">'+im+'</a>':im)+'</div>';
      }).join('')+'</div></div>'+
      (one?'':'<div class="menu-images__indicators">'+imgs.map(function(i,n){return '<button type="button" class="menu-images__indicator'+(n?'':' menu-images__indicator--active')+'" data-n="'+n+'" aria-label="image '+(n+1)+'"></button>';}).join('')+'</div>')+'</div>';
  }
  function bannerLogic(){
    var root=$('dash-banner');if(!root)return;
    var items=root.querySelectorAll('.menu-image'),dots=root.querySelectorAll('.menu-images__indicator');
    if(items.length<2)return;
    var cur=0,timer;
    function go(n){
      cur=(n+items.length)%items.length;
      Array.prototype.forEach.call(items,function(el,i){el.style.display=i===cur?'':'none';});
      Array.prototype.forEach.call(dots,function(el,i){el.classList.toggle('menu-images__indicator--active',i===cur);});
      var im=items[cur].querySelector('img'),bl=root.querySelector('.menu-images__blur');
      if(im&&bl)bl.style.backgroundImage='url("'+im.getAttribute('src')+'")';
    }
    function reset(){clearInterval(timer);timer=setInterval(function(){go(cur+1);},6000);}
    var l=root.querySelector('.menu-images__arrow--left'),r=root.querySelector('.menu-images__arrow--right');
    if(l)l.addEventListener('click',function(){go(cur-1);reset();});
    if(r)r.addEventListener('click',function(){go(cur+1);reset();});
    Array.prototype.forEach.call(dots,function(el){el.addEventListener('click',function(){go(+el.dataset.n);reset();});});
    reset();
  }

  function statusBox(s){
    var on=s.active?1:0;
    return '<div class="user-online-status"><div><div class="user-online-status__label">Online Friends</div><div class="user-online-status__value">0</div></div>'+
      '<div><div class="user-online-status__label">Current Games</div><div class="user-online-status__value">0</div></div>'+
      '<div><div class="user-online-status__label">Online Users</div><div class="user-online-status__value"><strong>'+on+'</strong></div></div>'+
      '<div class="user-online-status__chart" id="dash-chart"></div></div>';
  }
  function chart(){
    var box=$('dash-chart');if(!box)return;
    var w=box.clientWidth||260,h=box.clientHeight||60,pts=[],n=30,i;
    for(i=0;i<=n;i++){var x=i/n;pts.push([x*w,h*(0.45+0.2*Math.sin(x*6+1)+0.1*Math.sin(x*13))]);}
    var d='M0,'+h;pts.forEach(function(p){d+=' L'+p[0].toFixed(1)+','+p[1].toFixed(1);});d+=' L'+w+','+h+' Z';
    box.innerHTML='<svg width="'+w+'" height="'+h+'" aria-hidden="true"><path class="dash-graph-area" d="'+d+'"/></svg>';
  }

  function btn(href,label,icon,mod){
    return '<a href="'+href+'" target="_blank" rel="noreferrer" class="btn-home'+(mod?' btn-home--'+mod:'')+'"><span class="btn-home__text">'+label+'</span><span class="btn-home__icon"><i class="fas fa-fw fa-'+icon+'"></i></span></a>';
  }

  function show(s,hooks){
    var host=$('dash');if(!host)return;
    host.innerHTML=header(s)+
      '<div class="osu-page"><div id="dash-banner-slot"></div>'+
      '<div class="user-home"><div class="user-home__left-section"><div class="user-home__news"><h2 class="user-home__left-title">News</h2><div id="dash-news"><div class="dash-note">Loading…</div></div></div></div>'+
      '<div class="user-home__right-sidebar"><div class="user-home__status-box">'+statusBox(s)+'</div>'+
      '<div class="user-home__buttons">'+btn(OSU+'/home/download','Download osu!','download')+btn(OSU+'/home/support','Support osu!','heart','c-pink-darker')+btn(OSU+'/store','osu!store','shopping-cart','c-darkorange')+'</div>'+
      '<div id="dash-daily"></div>'+
      '<h3 class="user-home__beatmap-list-header">New Ranked Beatmaps</h3><div class="user-home__beatmapsets" id="dash-new"><div class="dash-note dash-note--side">Loading…</div></div>'+
      '<h3 class="user-home__beatmap-list-header">Popular Beatmaps<ul class="game-mode game-mode--inline" id="dash-modes">'+MODES.map(function(m,i){return '<li><button type="button" class="game-mode-link game-mode-link--inherit-size game-mode-link--button'+(i?'':' js-is-active')+'" data-mode="'+m[0]+'" title="'+m[1]+'"><span class="fal fa-extra-mode-'+m[0]+'"></span></button></li>';}).join('')+'</ul></h3>'+
      '<div class="user-home__beatmapsets" id="dash-popular"><div class="dash-note dash-note--side">Loading…</div></div></div></div></div>'+footer();
    chart();addEventListener('resize',chart);
    var cv=new Image(),cu='/site/cover?name='+encodeURIComponent(s.user);
    cv.onload=function(){document.documentElement.style.setProperty('--current-user-cover','url("'+cu+'")');};
    cv.src=cu;
    var fail='<div class="dash-note">Couldn\'t load this from osu.ppy.sh. It needs the osu! API credentials set up for the profile import and an internet connection.</div>';
    var failSide='<div class="dash-note dash-note--side">Unavailable right now.</div>';
    getJSON('menu').then(function(d){var h=banner(d);if(h){$('dash-banner-slot').innerHTML=h;bannerLogic();}}).catch(function(){});
    getJSON('news').then(function(d){$('dash-news').innerHTML=newsHtml(d);}).catch(function(){$('dash-news').innerHTML=fail;});
    var q='/beatmapsets?q=ranked%3E'+encodeURIComponent(new Date(Date.now()-30*864e5).toISOString().slice(0,10));
    getJSON('new').then(function(d){$('dash-new').innerHTML=listHtml(d,'new',q);}).catch(function(){$('dash-new').innerHTML=failSide;});
    getJSON('daily').then(function(d){
      if(!d||!d.set)return;
      $('dash-daily').innerHTML='<h3 class="user-home__beatmap-list-header"><a target="_blank" rel="noreferrer" href="'+OSU+'/wiki/en/Gameplay/Daily_challenge">Daily Challenge Beatmap</a></h3><div class="user-home__beatmapsets">'+setHtml(d.set,'daily',d.set.room?OSU+'/daily-challenge/'+d.set.room:null)+'</div>';
    }).catch(function(){});
    var mode='osu',mo={osu:0,taiko:1,fruits:2,mania:3};
    function popular(){
      var box=$('dash-popular');
      getJSON('popular','&mode='+mode).then(function(d){
        box.innerHTML=listHtml(d,'popular',q+'&sort=favourites_desc&m='+mo[mode]);
      }).catch(function(){box.innerHTML=failSide;});
    }
    $('dash-modes').addEventListener('click',function(e){
      var b=e.target.closest('button[data-mode]');if(!b||b.dataset.mode===mode)return;
      mode=b.dataset.mode;
      Array.prototype.forEach.call($('dash-modes').querySelectorAll('button'),function(x){x.classList.toggle('js-is-active',x===b);});
      $('dash-popular').innerHTML='<div class="dash-note dash-note--side">Loading…</div>';popular();
    });
    popular();
    var ub=$('dash-user-btn'),um=$('dash-user-menu');
    ub.addEventListener('click',function(e){e.stopPropagation();um.hidden=!um.hidden;ub.setAttribute('aria-expanded',String(!um.hidden));});
    document.addEventListener('click',function(e){if(!um.hidden&&!um.contains(e.target)){um.hidden=true;ub.setAttribute('aria-expanded','false');}});
    $('dash-switch').addEventListener('click',function(){um.hidden=true;if(hooks&&hooks.openLogin)hooks.openLogin();});
    $('dash-logout').addEventListener('click',function(){
      fetch('/site/logout',{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json','X-CSRF-Token':(hooks&&hooks.csrf)||s.csrf||''},body:'{}'})
        .then(function(){try{localStorage.removeItem('los_home_user');}catch(e){}location.reload();}).catch(function(){location.reload();});
    });
  }
  window.HomeDashboard={show:show};
})();
