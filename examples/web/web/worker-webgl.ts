// A tile workers' Web Worker (tile-worker.ts) on the module that draws with
// WebGL2, so that a browser without WebGPU downloads one module.

// The namespace, as in worker.ts.
import * as bindings from './pkg/webgl/mln_web.js';
import { serve } from './tile-worker.ts';

serve(bindings);
