--!strict
-- Registration by number (docs/design/content-model-v2.md §5.6 and §12):
-- the records of instant chip effect 3 (`off_80EC3F0[3]`) that something
-- still names by number run this module's `instant_chip`, the plus chips'
-- effect with its bonus and extra from the record's parameters
-- (lib/instant/plus):
--
--   MegaBstr, the pack's chip 0, which a zeroed chip field reads;
--   Atk+10's record, the dark chips' fifth substitute (Atk+10 itself is a
--     definition, chips/atk-10);
--   Atk+30 and Uninstll, SunMoon's Program Advance's ingredients;
--   DarkPlus, a dark chip with a substitute.
--
-- Navi+20, WhiCapsl, FinalGun and NumTrap are definitions and don't come
-- here. It goes when the ruleset names these by handle (phase C: the
-- zeroed chip, the substitutes, the dark chips) and the recipes name their
-- ingredients (step 10).

local plus = require("../../lib/instant/plus")

local by_number = {}

-- `sub_8010488`.
function by_number.instant_chip(user: Object, spec: InstantChipSpec)
    plus.by_record(user, spec)
end

return by_number
