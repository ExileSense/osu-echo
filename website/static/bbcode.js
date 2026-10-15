(function(){
  if(window.Bbcode)return;
  var OSU='https://osu.ppy.sh';
  function esc(s){return String(s==null?'':s).replace(/[&<>"']/g,function(c){return {'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c];});}

  function fixHref(u){
    u=String(u||'').trim();
    if(!u)return '';
    if(/^(javascript|data|vbscript):/i.test(u))return '';
    if(/^mailto:/i.test(u))return u;
    if(u.charAt(0)==='#')return u;
    var m=u.match(/^(?:https?:)?(?:\/\/)?(?:osu\.ppy\.sh)?(\/(?:beatmapsets|beatmaps|users|u|scores|rankings|wiki|community|home|forum|groups|help|store|legal|team|teams)(?:\/.*)?)$/i);
    if(m&&/^(?:https?:\/\/osu\.ppy\.sh|\/(?!\/))/i.test(u)){
      var p=m[1];
      if(/^\/(?:beatmapsets|beatmaps|users|scores)(?:\/|$)/i.test(p)&&!/\/(discussion|download|packs)/i.test(p))return p.replace(/^\/u\//,'/users/');
      if(/^\/u\//i.test(p))return p.replace(/^\/u\//,'/users/');
      return OSU+p;
    }
    if(/^\/(?!\/)/.test(u))return OSU+u;
    if(/^https?:\/\//i.test(u))return u;
    if(/^\/\//.test(u))return 'https:'+u;
    if(/^[\w.-]+\.[a-z]{2,}(\/|$)/i.test(u))return 'https://'+u;
    return '';
  }
  function isLocal(h){return h&&h.charAt(0)==='/'&&h.charAt(1)!=='/';}
  function aOpen(href,cls){
    var h=fixHref(href);if(!h)return '<span>';
    return '<a href="'+esc(h)+'"'+(cls?' class="'+cls+'"':'')+(isLocal(h)||h.charAt(0)==='#'?'':' target="_blank" rel="noreferrer noopener"')+'>';
  }
  function safeColour(c){c=String(c||'').trim();return /^(#[0-9a-f]{3,8}|[a-z]{3,20}|rgba?\([\d\s.,%]+\)|hsla?\([\d\s.,%]+\))$/i.test(c)?c:'';}
  function safeImg(u){u=String(u||'').trim();if(/^\/\//.test(u))u='https:'+u;return /^https?:\/\//i.test(u)?u:'';}

  var OK_TAGS={A:1,B:1,STRONG:1,I:1,EM:1,U:1,DEL:1,S:1,STRIKE:1,BR:1,P:1,DIV:1,SPAN:1,CENTER:1,H1:1,H2:1,H3:1,H4:1,H5:1,H6:1,UL:1,OL:1,LI:1,BLOCKQUOTE:1,PRE:1,CODE:1,IMG:1,BUTTON:1,HR:1,DETAILS:1,SUMMARY:1,TABLE:1,THEAD:1,TBODY:1,TR:1,TD:1,TH:1,SMALL:1,SUB:1,SUP:1,FONT:0};
  function cleanStyle(st){
    var out=[];String(st||'').split(';').forEach(function(d){
      var i=d.indexOf(':');if(i<0)return;var k=d.slice(0,i).trim().toLowerCase(),v=d.slice(i+1).trim();
      if((k==='color'&&safeColour(v))||(k==='font-size'&&/^[\d.]+(%|px|em|rem)$/.test(v))||(k==='text-align'&&/^(left|right|center|justify)$/.test(v))||(k==='font-weight'&&/^(bold|normal|\d{3})$/.test(v))||(k==='font-style'&&/^(italic|normal)$/.test(v))||(k==='text-decoration'&&/^[a-z\s-]+$/.test(v)))out.push(k+':'+v);
    });return out.join(';');
  }
  function walk(node,out){
    for(var n=node.firstChild;n;n=n.nextSibling){
      if(n.nodeType===3){out.push(esc(n.nodeValue));continue;}
      if(n.nodeType!==1)continue;
      var t=n.tagName;
      if(!(t in OK_TAGS)){
        if(/^(SCRIPT|STYLE|IFRAME|OBJECT|EMBED|LINK|META|FORM|INPUT|TEXTAREA|SELECT|SVG)$/.test(t))continue;
        walk(n,out);continue;
      }
      if(t==='FONT'){walk(n,out);continue;}
      var tag=t.toLowerCase(),attrs='',cls=(n.getAttribute('class')||'').replace(/[^\w\s-]/g,'').trim();
      if(t==='A'){
        var cl=cls.split(/\s+/).filter(function(c){return c==='js-spoilerbox__link'||c==='bbcode-spoilerbox__link';}).join(' ');
        if(/\bjs-spoilerbox__link\b/.test(cls)){out.push('<button type="button" class="js-spoilerbox__link bbcode-spoilerbox__link">');walk(n,out);out.push('</button>');continue;}
        var h=fixHref(n.getAttribute('href'));
        if(!h){out.push('<span>');walk(n,out);out.push('</span>');continue;}
        out.push('<a href="'+esc(h)+'"'+(cl?' class="'+cl+'"':'')+(isLocal(h)||h.charAt(0)==='#'?'':' target="_blank" rel="noreferrer noopener"')+'>');walk(n,out);out.push('</a>');continue;
      }
      if(t==='IMG'){
        var src=safeImg(n.getAttribute('data-src')||n.getAttribute('src'));if(!src)continue;
        out.push('<img class="bbcode-image" src="'+esc(src)+'" alt="'+esc(n.getAttribute('alt')||'')+'" loading="lazy" style="max-width:100%;height:auto">');continue;
      }
      if(t==='BUTTON'){ if(/js-spoilerbox__link/.test(cls)){out.push('<button type="button" class="js-spoilerbox__link bbcode-spoilerbox__link">');walk(n,out);out.push('</button>');}else walk(n,out);continue;}
      if(cls)attrs+=' class="'+esc(cls)+'"';
      var st=cleanStyle(n.getAttribute('style'));if(st)attrs+=' style="'+esc(st)+'"';
      if(t==='TD'||t==='TH'){var cs=parseInt(n.getAttribute('colspan'),10);if(cs>0&&cs<20)attrs+=' colspan="'+cs+'"';}
      if(t==='BR'||t==='HR'){out.push('<'+tag+'>');continue;}
      out.push('<'+tag+attrs+'>');walk(n,out);out.push('</'+tag+'>');
    }
  }
  function fromHtml(src){
    var doc=new DOMParser().parseFromString('<!doctype html><body>'+src,'text/html'),out=[];
    walk(doc.body,out);
    return '<div class="bbcode bbcode--normal-line-height">'+out.join('')+'</div>';
  }

  var SIMPLE={b:['<strong>','</strong>'],i:['<em>','</em>'],u:['<u>','</u>'],s:['<del>','</del>'],strike:['<del>','</del>'],
    centre:['<div class="bbcode__align-centre">','</div>'],center:['<div class="bbcode__align-centre">','</div>'],
    right:['<div class="bbcode__align-right">','</div>'],left:['<div class="bbcode__align-left">','</div>'],
    heading:['<h2>','</h2>'],notice:['<div class="well">','</div>'],c:['<code>','</code>'],code:['<pre>','</pre>'],
    spoiler:['<span class="bbcode-spoiler" style="background:currentColor" title="Spoiler">','</span>']};
  var BLOCK={centre:1,center:1,right:1,left:1,heading:1,notice:1,code:1,list:1,quote:1,box:1,spoilerbox:1,imagemap:1};
  var RAW={code:1,c:1,img:1,url:0,youtube:1,email:0,audio:1};
  function tokenize(s){
    var toks=[],re=/\[(\/?)([a-z*]+)(?:=("[^"]*"|'[^']*'|[^\]]*))?\]/gi,last=0,m;
    while((m=re.exec(s))){
      if(m.index>last)toks.push({t:'text',v:s.slice(last,m.index)});
      var arg=m[3]!=null?m[3].replace(/^["']|["']$/g,''):null;
      toks.push({t:m[1]?'close':'open',name:m[2].toLowerCase(),arg:arg,raw:m[0]});
      last=re.lastIndex;
    }
    if(last<s.length)toks.push({t:'text',v:s.slice(last)});
    return toks;
  }
  var KNOWN={b:1,i:1,u:1,s:1,strike:1,centre:1,center:1,right:1,left:1,heading:1,notice:1,c:1,code:1,spoiler:1,color:1,size:1,url:1,email:1,img:1,list:1,'*':1,quote:1,box:1,spoilerbox:1,profile:1,youtube:1,audio:1,imagemap:1};
  function parse(toks){
    var root={name:'#',kids:[]},stack=[root];
    function top(){return stack[stack.length-1];}
    toks.forEach(function(k){
      if(k.t==='text'){top().kids.push(k.v);return;}
      if(k.name==='*'){
        if(k.t==='open'){ var p=top(); if(p.name==='*')stack.pop(); var l=top(); if(l.name==='list'){var it={name:'*',kids:[]};l.kids.push(it);stack.push(it);} else l.kids.push(k.raw); }
        return;
      }
      if(!KNOWN[k.name]){top().kids.push(k.raw);return;}
      if(k.t==='open'){
        if(top().name==='code'||top().name==='c'||top().name==='img'){top().kids.push(k.raw);return;}
        var node={name:k.name,arg:k.arg,kids:[]};top().kids.push(node);stack.push(node);
      }else{
        var i=stack.length-1;while(i>0&&stack[i].name!==k.name){ if(stack[i].name==='*'){i--;continue;} i--; }
        if(i>0&&stack[i].name===k.name){stack.length=i;}
        else top().kids.push(k.raw);
      }
    });
    return root;
  }
  function text(n){return n.kids.map(function(k){return typeof k==='string'?k:text(k);}).join('');}
  function emit(n){
    if(typeof n==='string')return esc(n).replace(/\r?\n/g,'<br>');
    var kids=function(){return n.kids.map(emit).join('');};
    var nm=n.name;
    if(nm==='#')return kids();
    if(nm==='code'||nm==='c'){return SIMPLE[nm][0]+esc(text(n))+SIMPLE[nm][1];}
    if(SIMPLE[nm]&&nm!=='code'&&nm!=='c')return SIMPLE[nm][0]+kids()+SIMPLE[nm][1];
    switch(nm){
      case 'color':{var c=safeColour(n.arg);return c?'<span style="color:'+esc(c)+'">'+kids()+'</span>':kids();}
      case 'size':{var sz={50:50,85:85,100:100,150:150}[parseInt(n.arg,10)]||100;return '<span style="font-size:'+sz+'%">'+kids()+'</span>';}
      case 'url':{ var href=n.arg!=null?n.arg:text(n); var o=aOpen(href); var body=n.arg!=null?kids():esc(text(n)); return o+body+(o==='<span>'?'</span>':'</a>'); }
      case 'email':{var em=n.arg||text(n);return /^[^\s<>"']+@[^\s<>"']+$/.test(em)?'<a href="mailto:'+esc(em)+'">'+(n.arg?kids():esc(em))+'</a>':kids();}
      case 'profile':{var nm2=text(n);return '<a href="/users/'+encodeURIComponent(n.arg||nm2)+'" class="js-usercard">'+esc(nm2)+'</a>';}
      case 'img':{var u=safeImg(text(n).trim());return u?'<img class="bbcode-image" src="'+esc(u)+'" alt="" loading="lazy" style="max-width:100%;height:auto">':'';}
      case 'youtube':{var id=text(n).trim().replace(/[^\w-]/g,'');return id?'<a href="https://www.youtube.com/watch?v='+id+'" target="_blank" rel="noreferrer noopener">https://www.youtube.com/watch?v='+id+'</a>':'';}
      case 'audio':{var au=safeImg(text(n).trim());return au?'<audio controls preload="none" src="'+esc(au)+'"></audio>':'';}
      case 'quote':return '<blockquote>'+(n.arg?'<h4>'+esc(n.arg)+' wrote:</h4>':'')+kids()+'</blockquote>';
      case 'list':{var ord=n.arg&&n.arg!=='' ;var items=n.kids.filter(function(k){return typeof k!=='string'&&k.name==='*';});
        var pre=n.kids.filter(function(k){return typeof k==='string'&&k.trim();}).map(emit).join('');
        var li=items.map(function(it){return '<li>'+it.kids.map(emit).join('').replace(/^(<br>)+|(<br>)+$/g,'')+'</li>';}).join('');
        return (pre?'<p class="bbcode__list-title">'+pre+'</p>':'')+(ord?'<ol class="unordered">':'<ul>')+li+(ord?'</ol>':'</ul>');}
      case 'box':case 'spoilerbox':{
        var title=nm==='box'?(n.arg||'')   :'';
        return '<div class="js-spoilerbox bbcode-spoilerbox"><button type="button" class="js-spoilerbox__link bbcode-spoilerbox__link"><span class="bbcode-spoilerbox__link-icon"></span><span class="bbcode-spoilerbox__link-text">'+esc(title||'Spoiler')+'</span></button><div class="bbcode-spoilerbox__body">'+kids().replace(/^(<br>)+|(<br>)+$/g,'')+'</div></div>';}
      case '*':return kids();
      default:return kids();
    }
  }
  function tidy(h){
    return h.replace(/<br>\s*(<(?:\/?(?:div|h2|ul|ol|li|blockquote|pre|p)\b))/gi,'$1').replace(/(<\/(?:div|h2|ul|ol|li|blockquote|pre|h4|p)>)\s*<br>/gi,'$1');
  }
  function fromBbcode(src){
    var tree=parse(tokenize(String(src||'')));
    return '<div class="bbcode bbcode--normal-line-height">'+tidy(emit(tree))+'</div>';
  }

  function render(src){
    src=String(src==null?'':src);
    if(!src.trim())return '';
    var looksBb=/\[(?:b|i|u|s|url|img|box|spoilerbox|centre|center|right|color|size|list|quote|heading|notice|profile|youtube)(?:[\]=])/i.test(src);
    var looksHtml=/<\/?(?:div|a|br|p|span|img|strong|em|b|i|u|h\d|ul|ol|li|center|font|details|pre|code|blockquote)\b[^>]*>/i.test(src);
    if(looksHtml&&!(looksBb&&!/<(?:div|p|br|span|a)\b/i.test(src)))return fromHtml(src);
    return fromBbcode(src);
  }

  document.addEventListener('click',function(e){
    var l=e.target.closest&&e.target.closest('.js-spoilerbox__link');if(!l)return;
    var box=l.closest('.js-spoilerbox');if(!box)return;
    e.preventDefault();var open=box.classList.toggle('js-spoilerbox--open');var body=box.querySelector('.bbcode-spoilerbox__body');if(body)body.style.display=open?'block':'';
  });

  window.Bbcode={render:render,fromBbcode:fromBbcode,fromHtml:fromHtml,fixHref:fixHref};
})();
