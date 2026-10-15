(function(){
  function esc(s){return String(s==null?'':s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];});}
  function init(){
    if(document.getElementById('tp-footer'))return;
    fetch('/site/themes',{credentials:'same-origin'}).then(function(r){return r.json();}).then(function(d){
      if(!d||!d.themes||d.themes.length<2)return;
      var box=document.createElement('div');box.id='tp-footer';
      box.style.cssText='text-align:center;padding:14px 10px 26px;font-size:12px;color:#888;font-family:inherit;position:relative;z-index:1';
      box.innerHTML='<label>Site theme <select id="tp-select" aria-label="Site theme" style="margin-left:6px;font:inherit;padding:2px 6px;border-radius:4px;border:1px solid rgba(128,128,128,.5);background:rgba(128,128,128,.12);color:inherit">'+
        d.themes.map(function(t){return '<option value="'+esc(t.id)+'" style="color:#222" title="'+esc(t.description||'')+'">'+esc(t.name)+'</option>';}).join('')+'</select></label>';
      document.body.appendChild(box);
      var sel=box.querySelector('select');sel.value=d.current;
      sel.addEventListener('change',function(){
        document.cookie='site_theme='+encodeURIComponent(sel.value)+'; path=/; max-age=31536000; SameSite=Lax';
        location.reload();
      });
    }).catch(function(){});
  }
  if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',init);else init();
})();
