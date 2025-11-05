import * as universal from '../entries/pages/_layout.ts.js';

export const index = 0;
let component_cache;
export const component = async () => component_cache ??= (await import('../entries/pages/_layout.svelte.js')).default;
export { universal };
export const universal_id = "src/routes/+layout.ts";
export const imports = ["_app/immutable/nodes/0.DXrrkYIX.js","_app/immutable/chunks/CkBGWJc5.js","_app/immutable/chunks/Bc9KpfpO.js","_app/immutable/chunks/ByHw6-xk.js","_app/immutable/chunks/V8EXUeVp.js","_app/immutable/chunks/D9jxrO47.js","_app/immutable/chunks/twlSRkmj.js","_app/immutable/chunks/DZ_pRahH.js","_app/immutable/chunks/BC4PGmkJ.js"];
export const stylesheets = ["_app/immutable/assets/0.BLTRDIer.css"];
export const fonts = [];
