--!strict
-- The NaviCust programs' B+Back specials, BN5's weapon routines NaviStats +0x07 names (the patch cards' too, and as
-- their B buttons and charged shots):
--
-- - Shield (`megaman/shield`, routine 0x1F, 0x0800F8EA; the patch cards' `megaman/card-shield`, routine 0x62,
--   0x0800FDAC, the same code with its own charge row): BN5's guard (action 0x1F, lib/guard) as the Shield program's
--   (subtype 3: it heeds nothing outside the operation battle's mode, where a guarded hit puts 0x580 into the side's
--   gauge), no damage;
-- - Reflect (`megaman/reflect`, routine 0x61, 0x0800FD84; routine 0x20, 0x0800F910, is the same): BN5's guard as the
--   Reflect program's (subtype 4: a hit it guards from the front is fired back as the charged shot's projectile, 20
--   pixels up), 50 damage;
-- - AntiDmg (`megaman/anti-damage`, routine 0x21, 0x0800F938): its stance (action 0x3D, BN6's code:
--   @exelib/navicust/anti-damage), 50 damage.
--
-- The guards hold 20 ticks (their params words' first byte: 0x214, 0x114) in the Shield program's look (row 2) or the
-- Reflect program's (row 1); the navi's chips wait 40 ticks after them, 70 after AntiDmg's stance.

local anti_damage = require("@exelib/navicust/anti-damage")
local guard = require("../../../lib/guard/guard")

local guards = {}

local TICKS = 0x14
local LOCKOUT = 40

local SHIELD = guard.action {
    id = "megaman/shield/action",
    ticks = TICKS,
    look = guard.looks.navicust_shield,
    counter = { kind = "gauge" },
}
local REFLECT = guard.action {
    id = "megaman/reflect/action",
    ticks = TICKS,
    look = guard.looks.navicust_reflect,
    counter = { kind = "shot" },
}

local function raise(navi: Object, damage: number)
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = damage
    navi.attack_lockout = LOCKOUT
end

-- 0x0800F8EA.
guards.shield = define.weapon {
    id = "megaman/shield",
    -- The charge table's row 0x1F (0x0801CB02), by Charge 0 to 5: past 4 it reads on into row 0x20's first entry.
    charge_ticks = { 40, 40, 40, 40, 40, 40 },
    setup = function(navi: Object): Action
        raise(navi, 0)
        return SHIELD
    end,
}

-- 0x0800FDAC: the patch cards' Shield (B-Shield, B↓Shield), the same code as the program's, its own charge row.
guards.card_shield = define.weapon {
    id = "megaman/card-shield",
    -- The charge table's row 0x62, by Charge 0 to 5: past 4 it reads on into row 0x63's first entry.
    charge_ticks = { 40, 40, 40, 40, 40, 100 },
    setup = function(navi: Object): Action
        raise(navi, 0)
        return SHIELD
    end,
}

-- 0x0800FD84.
guards.reflect = define.weapon {
    id = "megaman/reflect",
    -- The charge table's row 0x61, by Charge 0 to 5: past 4 it reads on into row 0x62's first entry.
    charge_ticks = { 40, 40, 40, 40, 40, 40 },
    setup = function(navi: Object): Action
        raise(navi, 50)
        return REFLECT
    end,
}

-- 0x0800F938 and action 0x3D.
local made = anti_damage.make {
    weapon = "megaman/anti-damage",
    action = "megaman/anti-damage/action",
    -- The charge table's row 0x21, by Charge 0 to 5: past 4 it reads on into row 0x22's first entry.
    charge_ticks = { 60, 60, 60, 60, 60, 50 },
}
guards.anti_damage = made.weapon

return guards
