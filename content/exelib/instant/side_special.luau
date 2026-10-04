--!strict
-- Instant chip effect 11 (`sub_802E1BE`, `off_80EC3F0[11]`): the game
-- writes the attack's first parameter to the user's side state (+0x0B) and
-- a 480-tick timer (360 with per-player gauges, battle flag 0x40) to +0x2E,
-- which `sub_80107D4` counts down while the side state is active (battle
-- flag 0x40 mode). Nothing in battle reads either (docs/engine/
-- field-names.md, SideState), so the effect changes nothing observable.
--
-- No chip, weapon routine or link navi of BN6's names effect 11; BN5's
-- mode chips (CannMode to DrilMode, FinalGun) use it, the same code
-- (0x0802D62A), and their lab recordings match. Shared: either game's
-- effect 11.

local side_special = {}

-- `sub_802E1BE`: an `instant` hook.
function side_special.instant(_user: Object, _spec: InstantChipSpec) end

return side_special
