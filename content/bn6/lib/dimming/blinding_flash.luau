--!strict
-- Dimming chip subtype 2's controller (the original's effect object #0x23,
-- `sub_80E2E34`; docs/engine/dimming-chip-effects.md §4): the usual
-- dimming phases (@common/dimming), with an effect that has the user glow
-- white for 11 ticks, then flashes the screen and blinds every navi on the
-- enemy's area for 480 ticks (a hit of no damage), the user glowing 61
-- ticks more. No chip has subtype 2: nothing reaches it (unverified); a
-- chip would name `blinding_flash.hook` as its `dimming`.

local dimming_chips = require("@common/dimming")
local collision = require("../../rules/collision")
local regions = require("@common/regions")
local statuses = require("../../rules/status")

local blinding_flash = {}

local GLOW, FLASH = 0xA, 0x3C
local SOUND_FLASH = asset.sound("bn6:blinding-flash")
-- The hit (`byte_80E2F18`): the other side's whole area, no spark, an
-- attack that pierces invisibility and breaks guards (row 0x34), hitting
-- navis, no damage, status 0x30 (blind), hit modifier 0.
local IS = define.collision { id = "bn6:dimming/blinding-flash", side0 = 0x80004084, side1 = 0x40004084, row_offset = 0x1A0 }
local BLIND = statuses.blind_480

-- The user glows white this tick.
local function glow(me: Object)
    local user = me.related1
    if user then
        user.sprite.white = true
    end
end

-- `sub_80E2E78`, by phase: 11 ticks of glowing; the flash and the blinding
-- hit; 61 ticks.
local function effect(me: Object)
    if me.phase == 0 then
        -- sub_80E2E94
        if me.phase_init == 0 then
            me.timer = GLOW
            me.phase_init = 4
        end
        glow(me)
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = 4, 0
        end
        return
    elseif me.phase ~= 4 then
        error(string.format("subtype 2's effect: phase %#x reads past off_80E2E8C", me.phase))
    end
    -- sub_80E2EBC
    if me.phase_init == 0 then
        me.timer = FLASH
        battle.palette_flash(0, FLASH, true, false)
        battle.play_sound(SOUND_FLASH)
        local h = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = regions.area[1 - me.alliance],
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = IS,
            damage = 0,
            stamina = 0,
            hit_mod = 0,
            status = BLIND,
        })
        if h then
            h.run_while_dimmed = true
        end
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        dimming_chips.done(me)
        return
    end
    glow(me)
end

local kind = define.kind {
    id = "bn6:dimming/blinding-flash",
    pool = "effect",
    -- `sub_80E2E34`.
    update = dimming_chips.phases { name = "subtype 2", effect = effect },
}

-- The `dimming` hook (`sub_80E2F24`): the controller, of the user's side
-- (its facing not copied).
function blinding_flash.hook(user: Object, spec: DimmingChipSpec): Object?
    return dimming_chips.spawn(kind, user, spec, true)
end

return blinding_flash
