--!strict
-- Invisibl: its user turns invisible for six seconds
-- (objects/invisible).
--
-- Its record stays the pack's for now (it is the chip DarkInvs falls back
-- to without bug frags, which the ruleset names by number): this is its
-- `dimming` hook, which registration by number runs (chips/0b1-invisibl).

local invisible = require("../../objects/invisible/invisible")

-- 360 ticks (the record's first two parameter bytes, 104 and 1).
return { dimming = invisible.hook(360) }
