--!strict
-- WideSht and SuprSpr (the original's action 0x30, `sub_80ED55C`;
-- docs/engine/shot-chips.md §10): the shared action
-- (@exelib/widesht/action) with EXE6's wide shooter, attachment and wave.
--
-- Each chip states its action with its own wave (WideSht's ./init,
-- SuprSpr's chips/suprspr).

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

-- A wave variant.
widesht.variant = wave.variant

return widesht
