(function(){
  var $=function(id){return document.getElementById(id);};
  function esc(s){return String(s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];});}
  function wave(){
    var box=$('home-graph');if(!box)return;
    var w=box.clientWidth||1000,h=box.clientHeight||70,top=22;
    var pts=[],n=24;
    for(var i=0;i<=n;i++){var x=i/n;pts.push([x*w,top+(h-top)*(0.55+0.3*Math.sin(x*5.2+0.8)+0.15*Math.sin(x*11+2))]);}
    var d='M0,'+h;
    pts.forEach(function(p){d+=' L'+p[0].toFixed(1)+','+p[1].toFixed(1);});
    d+=' L'+w+','+h+' Z';
    box.innerHTML='<svg class="landing-graph" width="'+w+'" height="'+h+'" aria-hidden="true"><path class="landing-graph__area" d="'+d+'" style="stroke-linejoin:round"/></svg>';
  }
  wave();addEventListener('resize',wave);
  var box=$('home-login'),csrf='',names=[];
  function openLogin(switching){
    if(!box)return;
    $('home-login-title').textContent=switching?'Switch Profile':'Sign In To Proceed';
    $('home-login-error').textContent='';
    var dl=$('home-login-names');dl.innerHTML=names.map(function(n){return '<option value="'+esc(n)+'">';}).join('');
    box.hidden=false;
    var i=$('home-login-name');i.focus();i.select();
  }
  function closeLogin(){if(box)box.hidden=true;}
  if(box){
    var si=$('home-signin');
    if(si)si.addEventListener('click',function(e){e.preventDefault();box.hidden?openLogin(false):closeLogin();});
    document.addEventListener('mousedown',function(e){
      if(box.hidden)return;
      if(box.contains(e.target)||(si&&si.contains(e.target))||e.target.closest('[data-open-login]'))return;
      closeLogin();
    });
    document.addEventListener('keydown',function(e){if(e.key==='Escape')closeLogin();});
    $('home-login-form').addEventListener('submit',function(e){
      e.preventDefault();
      var btn=$('home-login-submit');btn.disabled=true;$('home-login-error').textContent='';
      fetch('/site/login',{method:'POST',credentials:'same-origin',headers:{'Content-Type':'application/json','X-CSRF-Token':csrf},body:JSON.stringify({username:$('home-login-name').value})})
        .then(function(r){return r.json().then(function(d){if(!r.ok)throw new Error(d.error||'Sign in failed.');return d;});})
        .then(function(){location.reload();})
        .catch(function(err){$('home-login-error').textContent=err.message;btn.disabled=false;});
    });
  }
  fetch('/site/session',{credentials:'same-origin'}).then(function(r){return r.json();}).then(function(s){
    csrf=s.csrf||'';names=s.profiles||[];
    var n=(s.profiles||[]).length;
    $('home-stats').innerHTML='<strong>'+n.toLocaleString()+'</strong> registered player'+(n===1?'':'s')+' on this local server';
    var acc=$('home-account');
    var modern=document.body&&document.body.getAttribute('data-theme')==='modern';
    if(modern){
      try{if(s.user)localStorage.setItem('los_home_user','1');else localStorage.removeItem('los_home_user');}catch(e){}
      if(s.user){
        document.documentElement.classList.add('home-loggedin');
        var sc=document.createElement('script');sc.src='/site/static/dashboard.js?v=6';
        sc.onload=function(){try{window.HomeDashboard.show(s,{openLogin:function(){openLogin(true);},csrf:csrf});}catch(e){document.documentElement.classList.remove('home-loggedin');}};
        sc.onerror=function(){document.documentElement.classList.remove('home-loggedin');};
        document.head.appendChild(sc);
      }else document.documentElement.classList.remove('home-loggedin');
    }
    if(s.user){
      acc.innerHTML='<a class="landing-nav__link landing-nav__link--bold" href="/users/'+encodeURIComponent(s.user)+'">'+esc(s.user)+'</a>'+
        '<a class="landing-nav__link" href="/profile#login" style="margin-left:24px">switch</a>';
    }
  }).catch(function(){$('home-stats').textContent='';document.documentElement.classList.remove('home-loggedin');});
})();
