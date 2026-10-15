(function(){
  if(window.SiteNav)return;
  var OSU='https://osu.ppy.sh';
  var S={session:null};
  function esc(s){return String(s==null?'':s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];});}
  function flagUrl(c){
    c=String(c||'').toUpperCase();if(!/^[A-Z]{2}$/.test(c)||c==='XX')return '';
    return OSU+'/assets/images/flags/'+c.split('').map(function(ch){return (0x1f1e6+ch.charCodeAt(0)-65).toString(16);}).join('-')+'.svg';
  }
  var LINKS=[['home','/'],['beatmaps','/beatmapsets'],['rankings',OSU+'/rankings/osu/performance'],['community',OSU+'/community/forums'],['store',OSU+'/home/support'],['help',OSU+'/wiki/en/Help_centre']];
  var AVATAR_FALLBACK="this.onerror=null;this.src='/site/static/vendor/avatar-guest@2x.01495bc4.png'";

  function extras(s){
    if(!s||!s.user)return '';
    var f=flagUrl(s.country);
    return (f?'<a class="sn-flag" href="/profile#settings" title="'+esc(s.country)+'"><img src="'+f+'" alt="'+esc(s.country)+'" width="28" height="20"></a>':'')+
      '<a class="sn-pill" href="/home/friends" title="messages and notifications" aria-label="messages and notifications"><span class="sn-pill__item"><i class="fas fa-comment-alt"></i><b>0</b></span><span class="sn-pill__item"><i class="fas fa-bell"></i><b>0</b></span></a>';
  }
  function userMenu(s){
    var nm=encodeURIComponent(s.user);
    return '<div class="dash-user"><button type="button" class="dash-avatar sn-avatar" data-sn="user" aria-haspopup="true" aria-expanded="false" title="'+esc(s.user)+'"><img src="/site/avatar?name='+nm+'" alt="" onerror="'+AVATAR_FALLBACK+'"></button>'+
      '<div class="dash-menu sn-menu" hidden><div class="dash-menu__name">'+esc(s.user)+'</div><a href="/users/'+nm+'">my profile</a><a href="/home/friends">friends</a><a href="/profile#settings">account settings</a>'+
      '<button type="button" data-sn="switch">switch profile</button><button type="button" data-sn="logout">sign out</button></div></div>';
  }
  function html(s,active){
    S.session=s||S.session||{};s=S.session;install();
    var ext=function(h){return h.charAt(0)==='/'?'':' target="_blank" rel="noreferrer"';};
    return '<div class="dash-top"><nav class="dash-nav" aria-label="Main navigation"><a class="dash-logo nav2__logo-link" href="/" aria-label="osu! home"><div class="nav2__logo nav2__logo--bg"></div><div class="nav2__logo"></div></a>'+
      '<div class="dash-links">'+LINKS.map(function(l){return '<a href="'+l[1]+'"'+ext(l[1])+(l[0]===active?' style="color:hsl(var(--hsl-h1))"':'')+'>'+l[0]+'</a>';}).join('')+
      '<a class="dash-icon" href="/beatmapsets" aria-label="search"><i class="fas fa-search"></i></a></div>'+
      '<div class="dash-right sn-right"><a class="dash-icon" href="https://twitter.com/osugame" target="_blank" rel="noreferrer" aria-label="Twitter"><i class="fab fa-twitter"></i></a>'+
      '<a class="dash-icon" href="'+OSU+'/home/support" target="_blank" rel="noreferrer" aria-label="Support"><i class="fas fa-heart"></i></a>'+
      (s.user?extras(s)+userMenu(s):'<a class="dash-icon sn-signin" href="/" data-sn="signin">sign in</a>')+'</div></nav></div>';
  }

  function ensureBox(){
    var a=document.getElementById('sn-login-anchor');if(a)return a;
    a=document.createElement('div');a.id='sn-login-anchor';a.className='sn-anchor';
    a.innerHTML='<div class="login-box login-box--landing" id="sn-login" hidden><div class="login-box__content">'+
      '<form class="login-box__section login-box__section--login" id="sn-login-form" autocomplete="off"><h2 class="login-box__row login-box__row--title" id="sn-login-title">Sign In To Proceed</h2>'+
      '<div class="login-box__row login-box__row--inputs"><input class="login-box__form-input" id="sn-login-name" name="username" placeholder="username" required maxlength="64" list="sn-login-names" autocomplete="username"><datalist id="sn-login-names"></datalist></div>'+
      '<div class="login-box__row login-box__row--error" id="sn-login-error" role="alert"></div>'+
      '<div class="login-box__row login-box__row--actions"><div class="login-box__action"><button class="btn-osu-big btn-osu-big--nav-popup" type="submit" id="sn-login-submit"><div class="btn-osu-big__content"><span class="btn-osu-big__left">Sign in</span><span class="fas fa-fw fa-sign-in-alt"></span></div></button></div></div></form>'+
      '<div class="login-box__section login-box__section--register"><h2 class="login-box__row login-box__row--title">Don\'t have an account?</h2><div class="login-box__row">Download osu! to create your own account!</div><div class="login-box__row login-box__row--actions"><div class="login-box__action"><a href="'+OSU+'/home/download" class="btn-osu-big btn-osu-big--nav-popup" target="_blank" rel="noreferrer"><div class="btn-osu-big__content"><span class="btn-osu-big__left">Download</span><span class="fas fa-fw fa-download"></span></div></a></div></div></div>'+
      '</div></div>';
    document.body.insertBefore(a,document.body.firstChild);
    a.querySelector('#sn-login-form').addEventListener('submit',function(e){
      e.preventDefault();
      var btn=document.getElementById('sn-login-submit'),err=document.getElementById('sn-login-error');btn.disabled=true;err.textContent='';
      fetch('/site/login',{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json','X-CSRF-Token':(S.session&&S.session.csrf)||''},body:JSON.stringify({username:document.getElementById('sn-login-name').value})})
        .then(function(r){return r.json().then(function(d){if(!r.ok)throw new Error(d.error||'Sign in failed.');return d;});})
        .then(function(){try{localStorage.setItem('los_home_user','1');}catch(e){}location.reload();})
        .catch(function(x){err.textContent=x.message;btn.disabled=false;});
    });
    return a;
  }
  function loadSession(){
    if(S.session&&S.session.csrf!=null)return Promise.resolve(S.session);
    return fetch('/site/session',{credentials:'same-origin'}).then(function(r){return r.json();}).then(function(s){S.session=s;return s;});
  }
  function openLogin(switching){
    loadSession().then(function(s){
      ensureBox();
      var box=document.getElementById('sn-login');
      document.getElementById('sn-login-title').textContent=switching?'Switch Profile':'Sign In To Proceed';
      document.getElementById('sn-login-error').textContent='';
      document.getElementById('sn-login-names').innerHTML=(s.profiles||[]).map(function(n){return '<option value="'+esc(n)+'">';}).join('');
      box.hidden=false;
      var i=document.getElementById('sn-login-name');i.focus();i.select();
    });
  }
  function closeLogin(){var b=document.getElementById('sn-login');if(b)b.hidden=true;}
  function closeMenu(){var m=document.querySelector('.sn-menu');if(m&&!m.hidden){m.hidden=true;var b=document.querySelector('.sn-avatar');if(b)b.setAttribute('aria-expanded','false');}}

  var installed=false;
  function install(){
    if(installed)return;installed=true;
    document.addEventListener('click',function(e){
      var t=e.target.closest&&e.target.closest('[data-sn]');
      var kind=t&&t.getAttribute('data-sn');
      if(kind==='signin'){e.preventDefault();var b=document.getElementById('sn-login');if(b&&!b.hidden)closeLogin();else openLogin(false);return;}
      if(kind==='user'){var m=t.parentNode.querySelector('.sn-menu');m.hidden=!m.hidden;t.setAttribute('aria-expanded',String(!m.hidden));return;}
      if(kind==='switch'){closeMenu();openLogin(true);return;}
      if(kind==='logout'){
        fetch('/site/logout',{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json','X-CSRF-Token':(S.session&&S.session.csrf)||''},body:'{}'})
          .then(function(){try{localStorage.removeItem('los_home_user');}catch(x){}location.reload();}).catch(function(){location.reload();});return;
      }
      if(!e.target.closest('.dash-user'))closeMenu();
      if(!e.target.closest('#sn-login-anchor'))closeLogin();
    });
    document.addEventListener('keydown',function(e){if(e.key==='Escape'){closeMenu();closeLogin();}});
  }
  window.SiteNav={html:html,extras:extras,userMenu:userMenu,openLogin:openLogin,closeLogin:closeLogin,setSession:function(s){S.session=s;},install:install};
})();
