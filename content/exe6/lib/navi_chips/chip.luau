--!strict
-- EXE6's navi chips (action 0x1B): @exelib/navi_chips/chip's. (EXE6 keeps the last navi chip used too,
-- `byte_203C960`, but nothing in EXE6 reads it.) A chip's `navi` is `chip.make { id, summon, ... }`.

local common = require("@exelib/navi_chips/chip")

local chip = {}


chip.PLAIN, chip.USER_STAYS, chip.USER_RETURNED = common.PLAIN, common.USER_STAYS, common.USER_RETURNED

chip.make = common.maker {}

return chip
