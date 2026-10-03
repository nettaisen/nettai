--!strict
-- Bass, the navi of the Bass chip (the original's actor object #0x4F,
-- `sub_80C3970`), brought by its navi chip controller: BN6's
-- (@common/bass/navi, which BN5's shares). He appears on his user's panel
-- with his cape (a form overlay), rises for 100 ticks, then fires 24
-- volleys of two shots (objects/panel-strike), 8 ticks apart, at the
-- columns ahead holding the other side's panels: each shot at a panel with
-- an enemy navi five times in eight (else, or if there is none, any panel
-- of those columns), none at a panel shot in the last 22 ticks. Then he
-- leaves, and the controller moves on. See docs/engine/chips.md §3.6.33.

local common = require("@common/bass/navi")
local panel_strike = require("../../objects/panel-strike/panel_strike")
local effects = require("../../lib/effects")

-- His shots: 20 ticks of warning, hit modifier 3, on solid panels only.
local SHOT: PanelStrike = { warning = 0x14, hit_mod = 3, solid_only = true }

return common.make {
    id = "bn6:bass/navi",
    sprite = asset.sprite("bn6:bass-anly"),
    -- His cape: a form overlay on his animation + 0x14, stepping while
    -- dimmed.
    cape_anim_offset = 0x14,
    raise_anim = 0xA,
    volley_anim = 0xC,
    ending = "timed",
    -- `sub_80C5F4A`: a panel strike running while dimmed (outlasting the
    -- battle's end).
    shoot = function(me: Object, x: number, y: number, damage: number)
        panel_strike.spawn(me, x, y, 0, SHOT, damage, 0, true)
    end,
    -- A shot's flash at his hand (effect 0x57).
    flash = define.effect { sprite = asset.sprite("bn6:hit-sparks"), anim = 8, palette = 0 },
    smoke = effects.smoke,
    appear = asset.sound("bn6:appear"),
}
