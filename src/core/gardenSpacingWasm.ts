import type { GardenCompanionInstance, GardenShape, GardenStarPlantInstance } from '../types/garden';
import { shapeCentroid, shapeToPolygon } from './geometry2d';

type PlantFlags = { allium: boolean; legume: boolean; fennel: boolean; wormwood: boolean };

interface GardenCoreExports extends WebAssembly.Exports {
  memory: WebAssembly.Memory;
  reserve_plants: (count: number) => number;
  reserve_site: (values: number) => number;
  relax_spacing: (treeCount: number) => void;
}

const RECORD_LEN = 6;
let core: GardenCoreExports | undefined;
let loading: Promise<void> | undefined;

export function loadGardenSpacing(): Promise<void> {
  if (!loading) {
    loading = import('../generated/garden-core.wasm?url')
      .then(async ({ default: url }) => {
        const response = await fetch(url);
        if (!response.ok) throw new Error(`Could not load garden-core: ${response.status}`);
        const { instance } = await WebAssembly.instantiate(await response.arrayBuffer());
        const exports = instance.exports as GardenCoreExports;
        if (
          !(exports.memory instanceof WebAssembly.Memory) ||
          typeof exports.reserve_plants !== 'function' ||
          typeof exports.reserve_site !== 'function' ||
          typeof exports.relax_spacing !== 'function'
        ) {
          throw new Error('Unexpected garden-core exports');
        }
        core = exports;
      })
      .catch((error: unknown) => {
        console.warn('Using TypeScript garden spacing because WebAssembly could not load.', error);
      });
  }
  return loading;
}

function encodeOutline(outline: GardenShape | null): number[] {
  if (!outline) return [];
  if (outline.kind === 'CIRCLE') return [1, outline.cxM, outline.cyM, outline.rM];
  return [outline.kind === 'RECT' ? 2 : 3, ...shapeCentroid(outline), ...shapeToPolygon(outline).flat()];
}

export function relaxSpacingInRust(
  companions: GardenCompanionInstance[],
  flags: PlantFlags[],
  trees: GardenStarPlantInstance[],
  outline: GardenShape | null,
): boolean {
  const engine = core;
  if (!engine || (outline?.kind === 'POLYGON' && outline.points.length < 3)) return false;
  if (companions.length === 0) return true;

  const outlineData = encodeOutline(outline);
  const siteSize = trees.length * 3 + outlineData.length;
  const offset = engine.reserve_plants(companions.length);
  const siteOffset = engine.reserve_site(siteSize);
  // Obtain views only after both reservations. Either can grow WebAssembly memory.
  const buffer = new Float64Array(engine.memory.buffer, offset, companions.length * RECORD_LEN);
  const site = new Float64Array(engine.memory.buffer, siteOffset, siteSize);
  // Each tree is x, y and a nitrogen-fixing flag, followed by the garden outline.
  for (const [i, tree] of trees.entries()) {
    site.set([tree.xM, tree.yM, Number(tree.starTree.category === 'NITROGEN_FIXING_TREE')], i * 3);
  }
  site.set(outlineData, trees.length * 3);
  // Record: x, y, spread, feature bits, UTF-16 hash, 31 raised to the ID length.
  for (const [index, companion] of companions.entries()) {
    const plantFlags = flags[index];
    const bits = Number(plantFlags.allium) | (Number(plantFlags.legume) << 1)
      | (Number(plantFlags.fennel) << 2) | (Number(plantFlags.wormwood) << 3);
    let hash = 0;
    let multiplier = 1;
    for (let i = 0; i < companion.plantId.length; i++) {
      hash = (Math.imul(hash, 31) + companion.plantId.charCodeAt(i)) | 0;
      multiplier = Math.imul(multiplier, 31);
    }
    buffer.set([companion.xM, companion.yM, companion.plant.spreadM, bits, hash, multiplier], index * RECORD_LEN);
  }

  engine.relax_spacing(trees.length);
  for (let i = 0; i < companions.length; i++) {
    companions[i].xM = buffer[i * RECORD_LEN];
    companions[i].yM = buffer[i * RECORD_LEN + 1];
  }
  return true;
}
