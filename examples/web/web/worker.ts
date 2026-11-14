// A tile workers' Web Worker (tile-worker.ts) on the module that draws with
// WebGPU.

// The module's namespace rather than its names, so that the page still builds
// on a module without tile workers (the `tile-workers` feature off), where
// the page starts none.
import * as bindings from "./pkg/mln_web.js";
import { serve } from "./tile-worker.ts";

serve(bindings);
