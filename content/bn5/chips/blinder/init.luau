--!strict
-- Blinder (BN5 chip 0x081): a flash that blinds every enemy navi: the dimming handler's subtype 8 (0x080E3140),
-- whose controller (effect object #0x23, 0x080E3050) is BN6's code that no BN6 chip uses (`sub_80E2E34`): the user
-- flashes white for 10 ticks, then a screen flash, the blinding hit over the other side's area (BN5's row 0x34,
-- blindness for 480 ticks) and 60 more ticks of the user flashing.
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local collision = require("../rules/collision")
local dimming_chips = require("@exelib/dimming")
local regions = require("@exelib/regions")
local statuses = require("../rules/status")

-- The phases of its effect.
local FLASH, BLIND = 0x0, 0x4
local SOUND = asset.sound("blinding-flash")

-- The user flashes white this tick.
local function whiten(me: Object)
    local user = me.related1
    if user then
        user.sprite.white = true
    end
end

-- BN6's `sub_80E2E94`: the user flashes for 10 ticks.
local function flash(me: Object)
    if me.phase_init == 0 then
        me.timer = 10
        me.phase_init = 4
    end
    whiten(me)
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me.phase, me.phase_init = BLIND, 0
    end
end

-- BN6's `sub_80E2EBC`: a 60-tick screen flash, the sound and the blinding hit over the other side's area (no damage,
-- no spark: `byte_80E2F18`, by side, BN5's 0x080E3134); 60 ticks of the user flashing, then undim.
local function blind(me: Object)
    if me.phase_init == 0 then
        me.timer = 60
        battle.palette_flash(0, 60, true, false)
        battle.play_sound(SOUND)
        local h = battle.hitbox(me, {
            panel_x = me.panel_x,
            panel_y = me.panel_y,
            element = me.element,
            z = 0,
            region = regions.area[1 - me.alliance],
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.collision_34,
            damage = 0,
            stamina = 0,
            hit_mod = 0,
            status = statuses.blind_480,
        })
        -- (`sub_80C53A6`: it resolves while dimmed.)
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
    whiten(me)
end

-- BN6's `sub_80E2E78`.
local function effect(me: Object)
    local phase = me.phase
    if phase == FLASH then
        flash(me)
    elseif phase == BLIND then
        blind(me)
    else
        error(string.format("Blinder's phase %#x reads past off_80E2E8C", phase))
    end
end

-- Its controller (effect object #0x23; BN6's `sub_80E2E34`).
local CONTROLLER = define.kind {
    id = "blinder/controller",
    pool = "effect",
    update = dimming_chips.phases { name = "Blinder", effect = effect },
}

return define.chip {
    id = "blinder",
    codes = { "A", "D", "H", "*" },
    element = "null",
    family = "invisible",
    class = "standard",
    rarity = 1,
    mb = 9,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
    dimming = function(user: Object, spec: DimmingChipSpec): Object?
        return dimming_chips.spawn(CONTROLLER, user, spec, true)
    end,
}
