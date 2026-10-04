--!strict
-- WideSht and SuprSpr (the original's action 0x30, `sub_80ED55C`;
-- docs/engine/shot-chips.md §10): the shared action
-- (@exelib/widesht/action) with BN6's wide shooter, attachment and wave.
--
-- These are their actions; the chips are ./chip and chips/suprspr
-- (WideSht is SuprSpr's Program Advance ingredient).

local attachment = require("../../objects/attachment")
local common = require("@exelib/widesht/action")
local wave = require("./wave")

local widesht = {}

widesht.STATE = common.STATE

local LOOK: WideShotLook = {
    -- The wide shooter (the original's attachment row 0x23).
    shooter = attachment.look { sprite = asset.sprite("wide-shooter"), palette = 0, attach_point = 12 },
    attachment = attachment.kind,
    wave = wave.kind,
}

function widesht.action(spec: WideShotSpec): Action
    return common.action(spec, LOOK)
end

widesht.actions = {
    widesht = widesht.action { id = "widesht/action", wave = wave.variants.wide, waves = 1 },
    suprspr = widesht.action { id = "suprspr/action", wave = wave.variants.super, waves = 3 },
}

return widesht
