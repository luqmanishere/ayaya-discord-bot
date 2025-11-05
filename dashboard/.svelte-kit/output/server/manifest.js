export const manifest = (() => {
function __memo(fn) {
	let value;
	return () => value ??= (value = fn());
}

return {
	appDir: "_app",
	appPath: "_app",
	assets: new Set(["robots.txt"]),
	mimeTypes: {".txt":"text/plain"},
	_: {
		client: {start:"_app/immutable/entry/start.CIsZp4EV.js",app:"_app/immutable/entry/app.f7bw2Q-T.js",imports:["_app/immutable/entry/start.CIsZp4EV.js","_app/immutable/chunks/BC4PGmkJ.js","_app/immutable/chunks/Bc9KpfpO.js","_app/immutable/chunks/ByHw6-xk.js","_app/immutable/entry/app.f7bw2Q-T.js","_app/immutable/chunks/Bc9KpfpO.js","_app/immutable/chunks/LlRm11He.js","_app/immutable/chunks/CkBGWJc5.js","_app/immutable/chunks/ByHw6-xk.js","_app/immutable/chunks/V8EXUeVp.js","_app/immutable/chunks/twlSRkmj.js"],stylesheets:[],fonts:[],uses_env_dynamic_public:false},
		nodes: [
			__memo(() => import('./nodes/0.js')),
			__memo(() => import('./nodes/1.js'))
		],
		remotes: {
			
		},
		routes: [
			
		],
		prerendered_routes: new Set(["/","/login"]),
		matchers: async () => {
			
			return {  };
		},
		server_assets: {}
	}
}
})();
