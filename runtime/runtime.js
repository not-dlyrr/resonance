/* Resonance DOM adapter v1. No Spotify bundle rewriting or privileged bridge. */
(() => {
  'use strict';
  if (window.Resonance) return;
  const config = window.__RESONANCE_CONFIG__;
  const disposers = [];
  const selectors = Object.freeze({ play:'[data-testid="control-button-playpause"]', next:'[data-testid="control-button-skip-forward"]', previous:'[data-testid="control-button-skip-back"]', title:'[data-testid="now-playing-bar"] [data-testid="context-item-link"], [data-testid="context-item-info-title"]', artist:'[data-testid="now-playing-bar"] [data-testid="context-item-info-artist"], [data-testid="context-item-info-subtitles"]' });
  function notify(text) {
    let el=document.getElementById('resonance-notice');
    if(!el){el=document.createElement('div');el.id='resonance-notice';el.setAttribute('role','status');document.body.append(el);}
    el.textContent=text;
    clearTimeout(notify.timer);notify.timer=setTimeout(()=>el.remove(),4500);
  }
  const api = Object.freeze({ version:'1.0.0', adapter:'spotify-1.3.4-dom-v1',config,selectors,
    on(target,event,handler){target.addEventListener(event,handler);const dispose=()=>target.removeEventListener(event,handler);disposers.push(dispose);return dispose;},
    button(action){const el=document.querySelector(selectors[action]);if(!el){notify('This control is unavailable on the current Spotify screen.');return false;}el.click();return true;},
    isPlaying(){const el=document.querySelector(selectors.play);return el?.getAttribute('aria-label')?.toLowerCase().includes('pause')||!!el?.querySelector('[data-testid="pause-icon"]');},
    track(){return {title:document.querySelector(selectors.title)?.textContent||'',artist:document.querySelector(selectors.artist)?.textContent||''};},
    notify, register(dispose){disposers.push(dispose);},report(id,error){console.error('Resonance extension',id,error);notify(`${id} failed. Use Recovery mode in Resonance.`);}
  });
  window.Resonance=api;
  document.documentElement.dataset.resonance=config.theme;
  // Compatibility subset only: no Player, Cosmos, React, webpack or Platform emulation.
  window.Spicetify = Object.freeze({showNotification:notify,LocalStorage:{get:key=>localStorage.getItem(key),set:(key,value)=>localStorage.setItem(key,String(value)),remove:key=>localStorage.removeItem(key)}});
  window.addEventListener('pagehide',()=>{for(const dispose of disposers.splice(0))try{dispose();}catch{}clearTimeout(notify.timer);},{once:true});
})();
