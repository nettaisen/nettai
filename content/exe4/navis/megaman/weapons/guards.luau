--!strict
-- The NaviCust programs' B+Left specials, EXE4's weapon routines NaviStats +0x0C names (the table at 0x0800CA7C):
--
-- - Shield (`megaman/shield`, routine 0x25, 0x0800CF88): EXE4's guard (action 0x25, lib/guard) as the Shield
--   program's (variant 3: it counters nothing), no damage, 20 ticks behind the shield's look row 2 (animation 4);
-- - Reflect (`megaman/reflect`, routine 0x26, 0x0800CFA8): the guard as the Reflect program's (variant 4: a hit it
--   turns aside is fired back as the charged shot's projectile), 100 damage, 20 ticks behind look row 1 (animation
--   2); its routine sets the B+Left special's cooldown to 40 as it starts (AIAttackVars +0x1A: the attack's end hands
--   no lockout on in EXE4, `reactions.attack_end_lockouts`);
-- - AntiMagc (`megaman/anti-magic`, routine 0x27, 0x0800D004): its stance (action 0x72, 0x080EE9EE), 100 damage: for
--   13 ticks the side's defensive chip is AntiDmg (chips/anti), with the stance's damage word and bonus (0x08022FDE),
--   then none (0x08022FFA); a hit AntiDmg catches meanwhile (or AntiSwrd's request) turns the stance into the trap's
--   counter (0x0800C780, as a trap's catch: AntiDmg's counter from the next tick, its name shown to the other player).

local antidmg = require("../../../chips/anti").antidmg
local collision = require("../../../rules/collision")
local guard = require("../../../lib/guard")
local projectile = require("../../../objects/projectile")
local shield = require("../../../objects/shield")
local sparks = require("../../../lib/sparks")

local guards = {}

local TICKS = 0x14
local REFLECT_COOLDOWN = 40
-- The charge table's rows 0x25 to 0x27 (0x080180B8), by Charge 0 to 4, Charge 5 reading on into the next row's first
-- entry: 60 ticks throughout. (B+Left specials, which nothing charges.)
local CHARGE = { 60, 60, 60, 60, 60, 60 }

local SHIELD = guard.action {
    id = "megaman/shield/action",
    ticks = TICKS,
    shield = shield.look { anim = 4 },
    counter = nil,
}
-- What Reflect fires back: the charged shot's projectile (row 6, 0x080ECEC2), 20 pixels up.
local REFLECTED = projectile.variant {
    id = "megaman/reflect/shot",
    collision = { is = collision.attack, hits = collision.hits_navis },
    hit_mod = 0,
    element = "null",
    spark = sparks.charged,
}
local REFLECTED_Z = 0x14_0000
local REFLECT = guard.action {
    id = "megaman/reflect/action",
    ticks = TICKS,
    shield = shield.look { anim = 2 },
    counter = function(me: Object, x: number, y: number, damage: number)
        projectile.spawn(me, x, y, { variant = REFLECTED, damage = damage, z = REFLECTED_Z })
    end,
}

local function raise(navi: Object, variant: number, damage: number)
    navi.attack_element = 0
    navi.attack_variant = variant
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = damage
end

-- 0x0800CF88.
guards.shield = new.weapon {
    id = "megaman/shield",
    charge_ticks = CHARGE,
    setup = function(navi: Object): Action
        raise(navi, 3, 0)
        return SHIELD
    end,
}

-- 0x0800CFA8.
guards.reflect = new.weapon {
    id = "megaman/reflect",
    charge_ticks = CHARGE,
    setup = function(navi: Object): Action
        raise(navi, 4, 100)
        navi.back_special_cooldown = REFLECT_COOLDOWN
        return REFLECT
    end,
}

-- AntiMagc's stance: the ticks the record stays (AIAttackVars+0x10: 12, and one more).
local ANTI_MAGIC_TICKS = 12

type AntiMagicState = { timer: number }

-- 0x080EE9EE: a caught hit's request (0x600) hands over to the trap's counter; else its one phase (0x080EEA1C): the
-- side's defensive chip AntiDmg with the stance's damage word and bonus, for 13 ticks, then none, and back to idle.
local ANTI_MAGIC = new.action {
    id = "megaman/anti-magic/action",
    state = { timer = "u16" },
    update = function(me: Object, s: AntiMagicState)
        if me:request("anti_damage_triggered") or me:request("anti_sword_triggered") then
            me:check_reactive_abort()
            return
        end
        if me.step ~= 0 then
            error(string.format("AntiMagc's stance: phase %#x reads past 0x080EEA18", me.step))
        end
        if me.step_init == 0 then
            me:set_status("using_action", true)
            battle.set_linked(me.alliance, {
                chip = antidmg,
                bonus = me.extra,
                damage = int.u32(me.attack_damage + me.hit_param * 0x10000),
                owner = me,
                object = nil,
            })
            me.step_init = 4
            s.timer = ANTI_MAGIC_TICKS
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            battle.clear_linked(me.alliance)
            me:exit_attack()
        end
    end,
}

-- 0x0800D004 (the charge table's row 0x27: 60 ticks throughout).
guards.anti_magic = new.weapon {
    id = "megaman/anti-magic",
    charge_ticks = CHARGE,
    setup = function(navi: Object): Action
        raise(navi, 0, 100)
        return ANTI_MAGIC
    end,
}

return guards
