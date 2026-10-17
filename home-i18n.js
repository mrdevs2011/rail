(function(){
var L=["uz","en","ru"],lang="uz";
try{var q=new URLSearchParams(location.search).get("lang")||localStorage.getItem("rail-lang");if(L.indexOf(q)>-1)lang=q}catch(e){}
var T={
uz:{menu_open:"Menyu",brand_sub:"uy",nav_docs:"Hujjatlar →",sb_home:"Bosh sahifa",nav_intro:"Kirish",nav_install:"O'rnatish",nav_feat:"Imkoniyatlar",nav_keys:"Klavishlar",sb_docs:"Hujjatlar",d_intro:"Kirish",d_inst:"O'rnatish",d_keys:"Klavishlar",title:"Lenta shaklidagi brauzer",lead:"Linux uchun klaviatura bilan boshqariladigan brauzer.",made:"Muhammdrasul",only_linux:"Hozir faqat Linux.",btn_dl:"Linux yuklab olish",btn_docs:"Hujjatlar",sec_install:"02 / O'rnatish",install_p:"Bir qatorli o'rnatish tavsiya etiladi.",copy:"nusxa",copied:"nusxalandi",step1:"Buyruqni terminalga qo'ying.",step2:"rail deb ishga tushiring.",step3:"Yangilash: rail --update.",sec_feat:"03 / Imkoniyatlar",f1_t:"2D lenta",f1_p:"Cheksiz gorizontal ustunlar.",f2_t:"Profillar",f2_p:"Alohida cookie va sessiya.",f3_t:"Klaviatura",f3_p:"Qisqa klavishlar.",f4_t:"Yengil",f4_p:"Rust + GTK4. Faqat Linux.",sec_keys:"04 / Qisqa klavishlar",th_key:"Klavish",th_act:"Vazifa",k_g:"URL satri",k_t:"Yangi ustun",k_v:"Vertikal stek",k_hl:"Chap / o'ng",k_jk:"Past / yuqori",k_p:"Profillar",k_w:"Yopish",keys_more:"To'liq ro'yxat — Hujjatlarda.",lbl_next:"Keyingi"},
en:{menu_open:"Menu",brand_sub:"home",nav_docs:"Docs →",sb_home:"Home",nav_intro:"Intro",nav_install:"Install",nav_feat:"Features",nav_keys:"Shortcuts",sb_docs:"Docs",d_intro:"Intro",d_inst:"Install",d_keys:"Shortcuts",title:"A browser that is a ribbon",lead:"Keyboard-driven Linux browser.",made:"Muhammdrasul",only_linux:"Linux only for now.",btn_dl:"Download for Linux",btn_docs:"Documentation",sec_install:"02 / Install",install_p:"One-line install is recommended.",copy:"copy",copied:"copied",step1:"Paste the command or download the binary.",step2:"Run rail.",step3:"Update: rail --update.",sec_feat:"03 / Features",f1_t:"2D ribbon",f1_p:"Infinite columns with stacks.",f2_t:"Profiles",f2_p:"Separate cookies and session.",f3_t:"Keyboard",f3_p:"Shortcuts for every action.",f4_t:"Light",f4_p:"Rust + GTK4. Linux only.",sec_keys:"04 / Shortcuts",th_key:"Key",th_act:"Action",k_g:"URL bar",k_t:"New column",k_v:"Vertical stack",k_hl:"Left / right",k_jk:"Down / up",k_p:"Profiles",k_w:"Close",keys_more:"Full list in Docs.",lbl_next:"Next"},
ru:{menu_open:"Меню",brand_sub:"дом",nav_docs:"Документация →",sb_home:"Главная",nav_intro:"Введение",nav_install:"Установка",nav_feat:"Возможности",nav_keys:"Клавиши",sb_docs:"Документация",d_intro:"Введение",d_inst:"Установка",d_keys:"Клавиши",title:"Браузер в виде ленты",lead:"Браузер для Linux с клавиатуры.",made:"Muhammdrasul",only_linux:"Пока только Linux.",btn_dl:"Скачать для Linux",btn_docs:"Документация",sec_install:"02 / Установка",install_p:"Установка одной командой.",copy:"копировать",copied:"скопировано",step1:"Вставьте команду в терминал.",step2:"Запустите rail.",step3:"Обновление: rail --update.",sec_feat:"03 / Возможности",f1_t:"2D лента",f1_p:"Горизонтальные столбцы.",f2_t:"Профили",f2_p:"Отдельные cookie и сессия.",f3_t:"Клавиатура",f3_p:"Горячие клавиши.",f4_t:"Лёгкий",f4_p:"Rust + GTK4. Только Linux.",sec_keys:"04 / Клавиши",th_key:"Клавиша",th_act:"Действие",k_g:"URL",k_t:"Новый столбец",k_v:"Стопка",k_hl:"Влево / вправо",k_jk:"Вниз / вверх",k_p:"Профили",k_w:"Закрыть",keys_more:"Полный список в документации.",lbl_next:"Далее"}
};
function t(k){var d=T[lang]||T.uz;return d[k]!=null?d[k]:(T.uz[k]||k)}
function apply(){
  document.documentElement.lang=lang;
  document.querySelectorAll("[data-t]").forEach(function(el){el.textContent=t(el.getAttribute("data-t"));});
  document.querySelectorAll(".lang button").forEach(function(b){b.classList.toggle("on",b.getAttribute("data-l")===lang);});
  try{localStorage.setItem("rail-lang",lang)}catch(e){}
  try{if(typeof BroadcastChannel!=="undefined")new BroadcastChannel("rail_sync_channel").postMessage({lang:lang})}catch(e){}
}
function boot(){
  document.querySelectorAll(".lang button").forEach(function(b){
    if(b.dataset.i18nBound==="1")return;
    b.dataset.i18nBound="1";
    b.type="button";
    b.addEventListener("click",function(e){e.preventDefault();lang=b.getAttribute("data-l");apply();});
  });
  var CMD="curl -fsSL https://raw.githubusercontent.com/mrdevs2011/rail/main/install.sh | bash";
  var box=document.getElementById("cmdbox"),btn=document.getElementById("copybtn");
  if(box&&btn){
    box.innerHTML="";
    box.appendChild(document.createTextNode(CMD+" "));
    box.appendChild(btn);
    btn.onclick=function(){
      if(navigator.clipboard)navigator.clipboard.writeText(CMD);
      btn.textContent=t("copied");
      setTimeout(function(){btn.textContent=t("copy")},1000);
    };
  }
  fetch("https://api.github.com/repos/mrdevs2011/rail/releases/latest").then(function(r){return r.json()}).then(function(j){
    if(j.tag_name){var e=document.getElementById("ver");if(e)e.textContent="v"+j.tag_name.replace(/^v/,"");}
  }).catch(function(){});
  var hb=document.querySelector(".hamburger"),sb=document.querySelector(".docs-sidebar"),ov=document.querySelector(".sidebar-overlay");
  if(hb&&sb){
    hb.onclick=function(){sb.classList.toggle("open");if(ov)ov.classList.toggle("open");};
    if(ov)ov.onclick=function(){sb.classList.remove("open");ov.classList.remove("open");};
  }
  var cur=document.getElementById("cur");
  if(cur&&matchMedia("(pointer:fine)").matches){
    var mx=0,my=0,cx=0,cy=0;
    addEventListener("pointermove",function(e){mx=e.clientX;my=e.clientY;});
    (function f(){cx+=(mx-cx)*.18;cy+=(my-cy)*.18;cur.style.left=cx+"px";cur.style.top=cy+"px";requestAnimationFrame(f);})();
  } else if(cur){cur.style.display="none";}
  try{
    if(typeof BroadcastChannel!=="undefined"){
      var bc=new BroadcastChannel("rail_sync_channel");
      bc.onmessage=function(ev){if(ev.data&&L.indexOf(ev.data.lang)>-1&&ev.data.lang!==lang){lang=ev.data.lang;apply();}};
    }
  }catch(e){}
  window.addEventListener("storage",function(ev){
    if(ev.key==="rail-lang"&&L.indexOf(ev.newValue)>-1&&ev.newValue!==lang){lang=ev.newValue;apply();}
  });
  apply();
}
if(document.readyState==="loading")document.addEventListener("DOMContentLoaded",boot);else boot();
})();
