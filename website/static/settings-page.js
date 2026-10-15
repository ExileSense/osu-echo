(function(){
  var dlg=document.getElementById('settings-dialog');
  if(!dlg||!dlg.classList.contains('settings-dialog'))return;
  var $=function(id){return document.getElementById(id);};
  Array.prototype.forEach.call(dlg.querySelectorAll('[data-close-settings]'),function(b){b.addEventListener('click',function(){dlg.close();});});
  Array.prototype.forEach.call(dlg.querySelectorAll('[data-jump]'),function(a){
    a.addEventListener('click',function(e){
      e.preventDefault();
      var el=$(a.getAttribute('data-jump'));if(el)el.scrollIntoView({behavior:'smooth',block:'start'});
      Array.prototype.forEach.call(dlg.querySelectorAll('.header-nav-v4__link--active'),function(x){x.classList.remove('header-nav-v4__link--active');});
      a.classList.add('header-nav-v4__link--active');
    });
  });
  var ta=$('settings-about'),pv=$('settings-signature-preview'),last=null;
  function preview(){
    if(!ta||!pv)return;
    var raw=ta.value.trim();last=ta.value;
    try{pv.innerHTML=raw&&typeof parseBBCode==='function'?parseBBCode(raw):'<span style="opacity:.6;font-style:italic">Signature preview</span>';}catch(e){}
  }
  function insert(open,close,ph){
    if(!ta)return;
    var s=ta.selectionStart,e=ta.selectionEnd,sel=ta.value.substring(s,e)||ph||'';
    ta.focus();ta.setRangeText(open+sel+(close||''),s,e,'select');preview();
  }
  var tb=$('settings-bbcode-toolbar');
  if(tb){
    Array.prototype.forEach.call(tb.querySelectorAll('button[data-tag]'),function(b){
      b.addEventListener('click',function(){
        switch(b.dataset.tag){
          case 'b':insert('[b]','[/b]','bold text');break;
          case 'i':insert('[i]','[/i]','italic text');break;
          case 's':insert('[strike]','[/strike]','strike text');break;
          case 'heading':insert('[heading]','[/heading]','Heading');break;
          case 'quote':insert('[quote]','[/quote]','Quote text');break;
          case 'list':insert('[list]\n[*] ','\n[*] Item 2\n[/list]','Item 1');break;
          case 'list-ol':insert('[list=1]\n[*] ','\n[*] Item 2\n[/list]','Item 1');break;
          case 'url':{var u=prompt('Link URL:','https://');if(u)insert('[url='+u+']','[/url]','link text');break;}
          case 'img':{var m=prompt('Image URL:','https://');if(m)insert('[img]'+m+'[/img]','','');break;}
        }
      });
    });
    var sz=$('bbcode-font-size');
    if(sz)sz.addEventListener('change',function(){if(sz.value){insert('[size='+sz.value+']','[/size]','text');sz.value='';}});
  }
  if(ta)ta.addEventListener('input',preview);
  setInterval(function(){if(dlg.open&&ta&&ta.value!==last)preview();},400);
  preview();
})();
