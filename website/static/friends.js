(function(){
  var OSU='https://osu.ppy.sh';
  var root=document.getElementById('fl-root');
  function esc(s){return String(s==null?'':s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];});}
  function $(sel,el){return (el||document).querySelector(sel);}
  function url(u){return /^https?:\/\//i.test(u||'')?u:'';}
  var NEXT=document.body.classList.contains('theme-next');
  var Y19=document.body.classList.contains('theme-2019');
  var st={user:null,csrf:'',friends:[],enriched:false,filter:'all',sort:'active',view:'list',loaded:false,error:'',menu:null};
  try{var v=localStorage.getItem('los_friends_view');if(v==='card'||v==='brick'||v==='list')st.view=v;}catch(e){}

  function ago(iso){
    var t=Date.parse(iso);if(isNaN(t))return '';
    var d=Math.max(0,Math.round((Date.now()-t)/1000)),u=[['year',31536000],['month',2592000],['day',86400],['hour',3600],['minute',60]];
    for(var i=0;i<u.length;i++)if(d>=u[i][1]){var n=Math.floor(d/u[i][1]);return n+' '+u[i][0]+(n>1?'s':'')+' ago';}
    return 'just now';
  }
  function flagUrl(c){
    c=String(c||'').toUpperCase();if(!/^[A-Z]{2}$/.test(c))return '';
    return OSU+'/assets/images/flags/'+c.split('').map(function(ch){return (0x1f1e6+ch.charCodeAt(0)-65).toString(16);}).join('-')+'.svg';
  }

  function nav(){return window.SiteNav?SiteNav.html(st.session||{user:st.user,csrf:st.csrf},'home'):'';}
  function header(){
    if(Y19&&window.Y19Header)return Y19Header.html(st.user,'home')+'<div class="y19l-spacer"></div>';
    if(NEXT&&window.NxHeader)return '<div class="nxb-top">'+NxHeader.html(st.user,[['home','/'],['friends','']],'home')+'</div>';
    var tabs=[['dashboard','/',false],['friends','/home/friends',true],['watchlists',OSU+'/home/follows',false],['account settings','/profile#settings',false]];
    return '<div class="bm-head"><div class="header-v4 header-v4--home">'+nav()+
      '<div class="header-v4__container header-v4__container--main"><div class="header-v4__bg-container"><div class="header-v4__bg u-current-user-cover"></div></div><div class="header-v4__content"><div class="header-v4__row header-v4__row--title"><div class="header-v4__icon"></div><div class="header-v4__title">dashboard</div></div></div></div>'+
      '<div class="header-v4__container"><div class="header-v4__content"><div class="header-v4__row header-v4__row--bar"><ul class="header-nav-v4 header-nav-v4--list">'+
      tabs.map(function(t){return '<li class="header-nav-v4__item"><a class="header-nav-v4__link'+(t[2]?' header-nav-v4__link--active':'')+'" href="'+t[1]+'"'+(t[1].charAt(0)==='/'?'':' target="_blank" rel="noreferrer"')+'><span class="fake-bold" data-content="'+esc(t[0])+'">'+esc(t[0])+'</span></a></li>';}).join('')+'</ul></div></div></div></div></div>';
  }
  function footer(){return '<div class="bm-foot"><a href="'+OSU+'/wiki/en/Rules" target="_blank" rel="noreferrer">Rules</a><a href="'+OSU+'/legal/terms" target="_blank" rel="noreferrer">Terms</a><a href="'+OSU+'/legal/privacy" target="_blank" rel="noreferrer">Privacy</a><a href="https://status.ppy.sh" target="_blank" rel="noreferrer">Server Status</a></div>';}

  function counts(){
    var on=0;st.friends.forEach(function(f){if(f.online===true)on++;});
    return {all:st.friends.length,online:on,offline:st.friends.length-on};
  }
  function visible(){
    var l=st.friends.filter(function(f){return st.filter==='all'||(st.filter==='online'?f.online===true:f.online!==true);});
    var cmp={
      active:function(a,b){
        if((a.online===true)!==(b.online===true))return a.online===true?-1:1;
        var ta=Date.parse(a.last_visit)||0,tb=Date.parse(b.last_visit)||0;
        return tb-ta||a.name.toLowerCase().localeCompare(b.name.toLowerCase());
      },
      rank:function(a,b){var ra=a.pp>0?a.rank:1e9,rb=b.pp>0?b.rank:1e9;return ra-rb||a.name.toLowerCase().localeCompare(b.name.toLowerCase());},
      name:function(a,b){return a.name.toLowerCase().localeCompare(b.name.toLowerCase());}
    }[st.sort];
    return l.sort(cmp);
  }
  function href(f){return f.local?'/users/'+encodeURIComponent(f.name):'/users/'+f.id;}
  function extAttr(f){return f.local?'':' target="_blank" rel="noreferrer"';}
  function badges(f){
    var h='';
    if(f.support>0){h+='<span class="fl-heart" title="osu!supporter">'+new Array(Math.min(3,f.support)+1).join('<span class="fas fa-heart"></span>')+'</span>';}
    h+='<span class="fl-friend" title="friend"><span class="fas fa-user"></span></span>';
    return '<span class="fl-icons">'+h+'</span>';
  }
  function groups(f){
    return (f.groups||[]).filter(function(g){return g.short;}).slice(0,2).map(function(g){return '<span class="fl-grp" style="--c:'+esc(/^#[0-9a-f]{3,8}$/i.test(g.colour)?g.colour:'#fff')+'" title="'+esc(g.name)+'">'+esc(g.short)+'</span>';}).join('');
  }
  function nxCard(f){
    var on=f.online===true,fu=flagUrl(f.country),status=f.online==null?'':(on?'Online':'Offline'),sub=(!on&&f.last_visit)?'last seen '+ago(f.last_visit):'';
    return '<div class="nxf-card'+(on?' nxf-card--on':'')+'" data-id="'+f.id+'"><a class="nxf-card__link" href="'+href(f)+'"'+extAttr(f)+'><img class="nxf-av" src="https://a.ppy.sh/'+f.id+'" alt="" loading="lazy">'+
      '<span class="nxf-who"><span class="nxf-name">'+esc(f.name)+'</span><span class="nxf-meta">'+(fu?'<img class="nxf-flag" src="'+fu+'" alt="'+esc(f.country)+'" title="'+esc(f.country)+'" width="20" height="14">':'')+groups(f)+(f.support>0?'<span class="nxf-heart fas fa-heart" title="osu!supporter"></span>':'')+'</span></span>'+
      '<span class="nxf-status"><i class="nxf-dot"></i><span>'+status+(sub?'<small>'+esc(sub)+'</small>':'')+'</span></span></a>'+
      '<button type="button" class="fl-menu-btn" data-menu="'+f.id+'" aria-label="menu"><span class="fas fa-ellipsis-v"></span></button></div>';
  }
  function card(f,cls){
    if(NEXT)return nxCard(f);
    var on=f.online===true,cover=url(f.cover),fu=flagUrl(f.country);
    var status=f.online==null?'':(on?'Online':'Offline');
    var sub=(!on&&f.last_visit)?'Last seen '+ago(f.last_visit):'';
    var flags='<div class="fl-flags">'+(fu?'<span class="flag-country" style="background-image:url(\''+fu+'\')" title="'+esc(f.country)+'"></span>':'')+(f.team&&url(f.team.flag)?'<span class="fl-team" style="background-image:url(\''+esc(f.team.flag)+'\')" title="'+esc(f.team.name)+'"></span>':'')+'</div>';
    return '<div class="user-card '+cls+' user-card--has-outline" data-id="'+f.id+'">'+
      '<div class="user-card__background-container">'+(cover?'<img class="user-card__background" src="'+esc(cover)+'" alt="" loading="lazy">':'')+'<div class="user-card__background-overlay'+(on?' user-card__background-overlay--online':'')+'"></div></div>'+
      '<a class="user-card__card" href="'+href(f)+'"'+extAttr(f)+'>'+
      '<div class="user-card__content user-card__content--details"><div class="user-card__avatar-space"><img class="user-card__avatar user-card__avatar--loaded" src="https://a.ppy.sh/'+f.id+'" alt="" loading="lazy"></div>'+
      '<div class="user-card__details">'+flags+'<div class="user-card__user"><div class="user-card__username-row"><span class="user-card__username">'+esc(f.name)+'</span>'+groups(f)+'</div></div>'+badges(f)+'</div></div>'+
      '<div class="user-card__content user-card__content--status"><div class="user-card__status"><div class="user-card__status-icon-container"><div class="user-card__status-icon'+(on?' user-card__status-icon--online':'')+'"></div></div>'+
      '<div class="user-card__status-messages"><div class="user-card__status-message">'+status+'</div>'+(sub?'<div class="user-card__status-message user-card__status-message--sub">'+esc(sub)+'</div>':'')+'</div></div>'+
      '<div class="user-card__icons"><button type="button" class="fl-menu-btn" data-menu="'+f.id+'" aria-label="menu"><span class="fas fa-ellipsis-v"></span></button></div></div></a></div>';
  }
  function brick(f){
    return '<div class="user-card-brick user-card-brick--friend"><span class="user-card-brick__group-bar"></span><a class="user-card-brick__link" href="'+href(f)+'"'+extAttr(f)+'>'+esc(f.name)+'</a></div>';
  }
  function body(){
    if(!st.user)return '<div class="fl-empty">Sign in to see your friends.<br><br><a href="/" style="color:hsl(var(--hsl-l1))">Go to the home page to sign in</a></div>';
    if(!st.loaded)return '<div class="fl-empty">Loading...</div>';
    if(st.error)return '<div class="fl-empty">'+esc(st.error)+'</div>';
    if(!st.friends.length)return '<div class="fl-empty">Your friends list is empty.<br>Add people in-game with <code>!friend add &lt;username&gt;</code> or pull your official list with <code>!friend sync</code>, then refresh this page.</div>';
    var c=counts(),list=visible();
    var f=function(k,label){return '<button type="button" class="fl-filter'+(st.filter===k?' on':'')+'" data-filter="'+k+'" style="--c:'+({all:'#fff',online:'#b3d944',offline:'#000'}[k])+'"><b>'+label+'</b><span>'+c[k]+'</span></button>';};
    var s=function(k,label){return '<button type="button" data-sort="'+k+'" class="'+(st.sort===k?'on':'')+'">'+(k==='rank'?'<span class="far fa-dot-circle" style="margin-right:4px"></span>':'')+label+'</button>';};
    var vm=function(k,ic){return '<button type="button" class="vm'+(st.view===k?' on':'')+'" data-view="'+k+'" aria-label="'+k+'"><span class="fas '+ic+'"></span></button>';};
    var items=!list.length?'<div class="fl-empty">Nobody here.</div>':st.view==='brick'?'<div class="fl-brick">'+list.map(brick).join('')+'</div>':
      '<div class="user-cards '+(st.view==='card'?'user-cards--card':'user-cards--list')+(NEXT?' nxf-cards':'')+'">'+list.map(function(x){return card(x,st.view==='card'?'':'user-card--list');}).join('')+'</div>';
    return '<div class="fl-filters">'+f('all','All')+f('online','Online')+f('offline','Offline')+'</div>'+
      '<div class="fl-toolbar"><span class="t">Sort by</span>'+s('active','Recently active')+s('rank','Rank')+s('name','Username')+'<span style="width:12px"></span>'+vm('card','fa-square')+vm('list','fa-bars')+vm('brick','fa-th')+'</div>'+
      '<div class="fl-list">'+items+'</div>'+(st.enriched?'':'<div class="fl-note">Online status needs osu! API credentials in your config; showing saved data only.</div>');
  }
  function draw(){
    root.innerHTML=header()+'<div class="osu-page"><div class="fl-main" id="fl-main">'+body()+'</div></div>'+footer();
    document.title='friends · dashboard | osu!';
  }
  function redrawMain(){var m=$('#fl-main');if(m)m.innerHTML=body();}

  function removeFriend(id){
    fetch('/site/friends/remove',{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json','X-CSRF-Token':st.csrf},body:JSON.stringify({id:id})}).then(function(r){return r.json().catch(function(){return {};}).then(function(j){if(!r.ok)throw new Error(j.error||String(r.status));});}).then(function(){
      st.friends=st.friends.filter(function(f){return f.id!==id;});closeMenu();redrawMain();
    }).catch(function(e){alert(e.message||'Could not remove friend.');});
  }
  function closeMenu(){var p=$('.fl-pop');if(p)p.remove();st.menu=null;}
  document.addEventListener('click',function(e){
    var mb=e.target.closest('[data-menu]');
    if(mb){
      e.preventDefault();e.stopPropagation();
      var id=Number(mb.getAttribute('data-menu')),was=st.menu===id;closeMenu();if(was)return;
      var f=st.friends.filter(function(x){return x.id===id;})[0];if(!f)return;
      var card=mb.closest('.user-card,.nxf-card');st.menu=id;
      var pop=document.createElement('div');pop.className='fl-pop';
      pop.innerHTML='<a href="'+href(f)+'"'+extAttr(f)+'>view profile</a><a href="'+OSU+'/users/'+f.id+'" target="_blank" rel="noreferrer">open on osu.ppy.sh</a><button type="button" class="danger" data-remove="'+id+'">remove friend</button>';
      card.appendChild(pop);return;
    }
    var rm=e.target.closest('[data-remove]');
    if(rm){removeFriend(Number(rm.getAttribute('data-remove')));return;}
    if(!e.target.closest('.fl-pop'))closeMenu();
    var ft=e.target.closest('[data-filter]');if(ft){st.filter=ft.getAttribute('data-filter');redrawMain();return;}
    var so=e.target.closest('[data-sort]');if(so){st.sort=so.getAttribute('data-sort');redrawMain();return;}
    var vw=e.target.closest('[data-view]');if(vw){st.view=vw.getAttribute('data-view');try{localStorage.setItem('los_friends_view',st.view);}catch(x){}redrawMain();}
  });

  function boot(){
    draw();
    fetch('/site/session',{credentials:'same-origin'}).then(function(r){return r.json();}).then(function(s){
      st.user=s&&s.user||null;st.csrf=s&&s.csrf||'';st.session=s||null;draw();
      if(st.user){var cu='/site/cover?name='+encodeURIComponent(st.user),im=new Image();im.onload=function(){document.documentElement.style.setProperty('--current-user-cover','url("'+cu+'")');};im.src=cu;}
      if(!st.user)return;
      return fetch('/site/friends',{credentials:'same-origin'}).then(function(r){return r.json().catch(function(){return {};}).then(function(j){if(!r.ok)throw new Error(j.error||String(r.status));return j;});}).then(function(d){
        st.friends=d.friends||[];st.enriched=!!d.enriched;st.loaded=true;redrawMain();
      });
    }).catch(function(e){st.loaded=true;st.error=e.message||'Could not load friends.';redrawMain();});
  }
  boot();
})();
