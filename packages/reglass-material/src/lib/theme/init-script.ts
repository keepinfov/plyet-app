// re:glass material — render-blocking theme bootstrap.
//
// themeInitScript(prefix) returns a minified snippet of plain JS meant to run
// in a blocking <script> in <head> BEFORE first paint, so all four theming
// attributes are present before any styled content renders (prevents a flash
// of unthemed/wrong-accent content). Mirrors ThemeController's storage keys.

export function themeInitScript(prefix = 'rg'): string {
  return (
    `try{var p=${JSON.stringify(prefix)},d=document.documentElement,g=function(k){return localStorage.getItem(p+'-'+k)};` +
    `d.setAttribute('data-theme',g('theme')||'light');` +
    `d.setAttribute('data-accent',g('accent')||'blue');` +
    `d.setAttribute('data-blur',g('blur')==='off'?'off':'on');` +
    `d.setAttribute('data-transparency',g('transparency')==='off'?'off':'on');` +
    `}catch(e){}`
  );
}
