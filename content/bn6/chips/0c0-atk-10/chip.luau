--!strict
-- Registration by number (docs/design/content-model-v2.md §5.6, "As
-- built"): the records of instant chip effect 3 (`off_80EC3F0[3]`: MegaBstr,
-- Atk+30, WhiCapsl, Uninstll, DarkPlus and the special chips of the
-- subtype, which stay records while something names them by number) run
-- this module's `instant_chip`, the plus chips' effect with its bonus and
-- extra from the record's parameters (lib/instant/plus). Atk+10 and
-- Navi+20 are definitions (chips/atk-10, chips/navi-20) and don't come
-- here. It goes when the rest are definitions.

local plus = require("../../lib/instant/plus")

local by_number = {}

-- `sub_8010488`.
function by_number.instant_chip(user: Object, spec: InstantChipSpec)
    plus.by_record(user, spec)
end

return by_number
