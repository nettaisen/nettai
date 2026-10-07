--!strict
-- The NaviCust programs' B+Left specials, EXE4's weapon routines NaviStats +0x0C names (the table at 0x0800CA7C):
--
-- - Shield (`megaman/shield`, routine 0x25, 0x0800CF88): EXE4's guard (action 0x25, lib/guard) as the Shield
--   program's (variant 3: it counters nothing), no damage, 20 ticks behind the shield's look row 2 (animation 4);
-- - Reflect (`megaman/reflect`, routine 0x26, 0x0800CFA8): the guard as the Reflect program's (variant 4: a hit it
--   turns aside is fired back as the charged shot's projectile), 100 damage, 20 ticks behind look row 1 (animation
--   2); its routine sets the B+Left special's cooldown to 40 as it starts (AIData +0x1A: the attack's end hands no
--   lockout on in EXE4, `reactions.attack_end_lockouts`).
--
-- (AntiMagc, routine 0x27, waits on the AntiDmg chip whose trap it arms: docs/design/exe4-map.md §18 item 53.)

local guard = require("../../../lib/guard")
local shield = require("../../../objects/shield")

local guards = {}

local TICKS = 0x14
local REFLECT_COOLDOWN = 40
-- The charge table's rows 0x25 and 0x26 (0x080180B8), by Charge 0 to 4, Charge 5 reading on into the next row's
-- first entry: 60 ticks throughout. (B+Left specials, which nothing charges.)
local CHARGE = { 60, 60, 60, 60, 60, 60 }

local SHIELD = guard.action {
    id = "megaman/shield/action",
    ticks = TICKS,
    shield = shield.look { anim = 4 },
    counter = { kind = "none" },
}
local REFLECT = guard.action {
    id = "megaman/reflect/action",
    ticks = TICKS,
    shield = shield.look { anim = 2 },
    counter = { kind = "shot" },
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

return guards
