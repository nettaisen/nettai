--!strict
-- The second WhiCapsl (chip 0x17E, a special chip no folder holds): a
-- dimming chip that turns its user invisible for two seconds
-- (objects/invisible).
--
-- Its record stays the pack's for now (the dimming service finds the
-- chips no one can cut in on by number, and it is one): this is its
-- `dimming` hook, which registration by number runs (chips/0b1-invisibl).

local invisible = require("../../objects/invisible/invisible")

-- 120 ticks (the record's first two parameter bytes, 120 and 0).
return { dimming = invisible.hook(120) }
