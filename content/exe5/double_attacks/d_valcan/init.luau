--!strict
-- D-VALCAN (the DS's chip 0x163; Double Vulcan), the Double Attack of MegaMan and NapalmMan (docs/design/exe5ds-map.md
-- §5, §5.2: its controller the DS's effect object 0x9A, its navis its T1 0x38, which the controller tells of each
-- other). Each fires 30 of the Vulcans' bullets (EXE5's attack object 0x12, objects/bullet) down the user's row, each
-- at a height drawn from RNG2: MegaMan from his vulcan, one every 9 ticks, NapalmMan from his arm, one every 9; each
-- bullet bursts on the first thing in its row, hitting its panel and the one behind (the DS's bullet rows 0x15 and
-- 0x14, its own, as the Vulcans' row 0x0C and NapalmSoul's charged shot's 0x11, but hitting while the battle is dimmed:
-- the DS's burst gives those two rows back the bit EXE5's retype leaves out). MegaMan waits for NapalmMan to be done.
-- The chip's record and name are the DS's (written by tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs);
-- its record's use is the cannon's (action 0x14, subtype 0: the DS copied Cannon's record), which nothing starts: no
-- folder or hand holds it.

local attachment = require("../objects/attachment")
local buster = require("../lib/buster")
local bullet = require("../objects/bullet")
local bullets = require("@exelib/bullet")
local cannon_chips = require("../chips/cannon")
local double_attack = require("../lib/double_attack")
local megaman = require("../navis/megaman").megaman
local napalmman = require("../navis/napalmman").napalmman
local regions = require("@exelib/regions")

type State = DoubleAttackNavi

local CHARGE_TICKS = 0x3C
local AFTER_TICKS = 0xA
local ANIM_IDLE = 0
-- 30 bullets each; between two, 8 more ticks (MegaMan's count, NapalmMan's two phases).
local SHOTS = 0x1E
local SOUND_SHOT = asset.sound("col-army-2")
-- The bullets' heights, by an RNG2 draw's low two bits (0x02139080, 0x021390A0: 8, 16, 24 and 32 pixels).
local HEIGHTS = { 0x08_0000, 0x10_0000, 0x18_0000, 0x20_0000 }
-- The DS's bullet rows 0x15 (MegaMan's, as the Vulcans' 0x0C) and 0x14 (NapalmMan's, as NapalmSoul's 0x11): the panel
-- found and the one behind, staggering, the hit shown once at the bullet (effect 0x20 and 0x58: the vulcan hit in its
-- animations 0 and 1).
local MEGAMAN_SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 1,
    effect = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 0 },
    single = true,
    hits_while_dimmed = true,
}
local NAPALMMAN_SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 1,
    effect = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 1, palette = 0 },
    single = true,
    hits_while_dimmed = true,
}
-- MegaMan's vulcan (the attachment table's row 0x0B, the Vulcans' gun at his attach point 14): raised in his
-- animation 0xA, firing in 0xD; its own animation one on as it fires, another as it stops.
local VULCAN = attachment.look { sprite = asset.sprite("vulcan"), palette = 0, attach_point = 14 }
local ANIM_HOLD, ANIM_FIRE = 0xA, 0xD
local RAISE_TICKS, BETWEEN, LOWER_TICKS = 1, 8, 0xA
-- NapalmMan's firing animation, and the muzzle flash at his arm (row 0x0D) on his second tick; 5 more ticks between.
local ANIM_NAPALM_FIRE = 0xE
local NAPALM_FIRES_AT, NAPALM_FIRE_TICKS, NAPALM_BETWEEN = 1, 2, 5
-- The actions: appear, charge, attack (by phase), after, leave.
local APPEAR, CHARGE, ATTACK, AFTER, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_80C6ADA` (0x020A1FCC): a bullet of `variant` from the panel ahead, at a height drawn from RNG2, null, with its
-- damage word, running while dimmed.
local function fire(me: Object, variant: BulletVariant)
    local z = HEIGHTS[bit32.band(battle.rng_positive(), 3) + 1]
    local word = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local o = bullets.fire(bullet.kind, me, me.panel_x + me.facing, me.panel_y, z, variant, 0, word)
    if o then
        o.run_while_dimmed = true
    end
end

local function appear(me: Object)
    double_attack.appear(me, CHARGE, LEAVE)
end

-- 0x020A5B80: MegaMan charges; his destination the panel ahead in the top row (the DS also sets its +0x70, which
-- nothing reads).
local function megaman_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    me:set_action(ATTACK)
end

-- 0x020A5DBC: NapalmMan charges; 30 bullets to fire.
local function napalm_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.timer2 = SHOTS
    me:set_action(ATTACK)
end

-- 0x020A5C14's phases: the vulcan raised (0x020A5C24, 2 ticks), the bullets (0x020A5C58), lowered (0x020A5CCC, 11
-- ticks).
local function megaman_attack(me: Object)
    local s = me.state :: State
    local p = me.phase
    if p == 0x0 then
        if me.phase_init == 0 then
            me:set_animation(ANIM_HOLD)
            attachment.attach(me, VULCAN, "held", { anim = 0, while_dimmed = true })
            me.phase_init = 4
            me.timer = RAISE_TICKS
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = 0x4, 0
        end
    elseif p == 0x4 then
        if me.phase_init == 0 then
            me.timer2 = SHOTS
            local gun = s.held :: Object
            gun.anim += 1
            me:set_animation(ANIM_FIRE)
            me.timer = 0
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t >= 0 then
            return
        end
        me.timer = BETWEEN
        fire(me, MEGAMAN_SHOT)
        battle.play_sound(SOUND_SHOT)
        local n = me.timer2 - 1
        me.timer2 = n
        if n <= 0 then
            me.phase, me.phase_init = 0x8, 0
        end
    elseif p == 0x8 then
        if me.phase_init == 0 then
            local gun = s.held :: Object
            gun.anim += 1
            me:set_animation(ANIM_HOLD)
            me.timer = LOWER_TICKS
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me:set_action(AFTER)
        end
    else
        error(string.format("D-VALCAN's MegaMan: phase %#x reads past his table (0x02139074)", p))
    end
end

-- 0x020A5E44's phases: a bullet (0x020A5E54: his firing animation, on its second tick the bullet, its sound and the
-- muzzle flash; 3 ticks), then 6 ticks (0x020A5EA0: the flash let go), 30 times.
local function napalm_attack(me: Object)
    local s = me.state :: State
    local p = me.phase
    if p == 0x0 then
        if me.phase_init == 0 then
            me.anim, me.anim_loaded = ANIM_NAPALM_FIRE, 0xFF
            me.phase_init = 4
            me.timer = 0
        end
        if me.timer == NAPALM_FIRES_AT then
            battle.play_sound(SOUND_SHOT)
            fire(me, NAPALMMAN_SHOT)
            double_attack.detach(s.held)
            attachment.attach(me, buster.flash, "held", { anim = 0, while_dimmed = true })
        end
        local t = me.timer + 1
        me.timer = t
        if t > NAPALM_FIRE_TICKS then
            me.phase, me.phase_init = 0x4, 0
        end
    elseif p == 0x4 then
        if me.phase_init == 0 then
            me.timer = NAPALM_BETWEEN
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t >= 0 then
            return
        end
        s.held = nil
        local n = me.timer2 - 1
        me.timer2 = n
        if n > 0 then
            me.phase, me.phase_init = 0x0, 0
        else
            me:set_action(AFTER)
        end
    else
        error(string.format("D-VALCAN's NapalmMan: phase %#x reads past his table (0x02139098)", p))
    end
end

-- 0x020A5CFC: MegaMan waits for NapalmMan to be done; then idle, his vulcan let go, 10 ticks, and he leaves.
local function megaman_after(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if double_attack.partner(me).action ~= AFTER then
            return
        end
        me.anim = ANIM_IDLE
        s.held = nil
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(LEAVE)
    end
end

-- 0x020A5F0C: NapalmMan idle, his flash let go, 10 ticks, and he leaves.
local function napalm_after(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = ANIM_IDLE
        s.held = nil
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(LEAVE)
    end
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 129,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "d-valcan",
        damage = { 2, 2, 2, 2, 2, 3, 4, 5, 6, 6 },
        roles = {
            -- 0x020A5B1C's table: MegaMan, (10, 10) from the user's spot.
            {
                navi = megaman,
                dx = 10,
                dy = 10,
                actions = {
                    [APPEAR] = appear,
                    [CHARGE] = megaman_charge,
                    [ATTACK] = megaman_attack,
                    [AFTER] = megaman_after,
                    [LEAVE] = function(me: Object)
                        double_attack.leave(me, true)
                    end,
                },
            },
            -- 0x020A5D64's: NapalmMan, (-10, -10).
            {
                navi = napalmman,
                dx = -10,
                dy = -10,
                actions = {
                    [APPEAR] = appear,
                    [CHARGE] = napalm_charge,
                    [ATTACK] = napalm_attack,
                    [AFTER] = napalm_after,
                    [LEAVE] = function(me: Object)
                        double_attack.leave(me, false)
                    end,
                },
            },
        },
    },
}

return { ["d-valcan"] = chip }
