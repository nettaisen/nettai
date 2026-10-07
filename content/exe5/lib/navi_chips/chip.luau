--!strict
-- EXE5's navi chips (action 0x41): @exelib/navi_chips/chip's, which keep a chip of the navi chips' block as the last
-- navi chip used (rules/api), for DethPhnx. A chip's `navi` is `chip.make { id, summon, ... }`.

local api = require("../../rules/api")
local common = require("@exelib/navi_chips/chip")

local chip = {}


chip.PLAIN, chip.USER_STAYS, chip.USER_RETURNED = common.PLAIN, common.USER_STAYS, common.USER_RETURNED

chip.make = common.maker { keep = api.keep_navi_chip }

return chip
