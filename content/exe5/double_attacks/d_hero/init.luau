--!strict
-- D-HERO (the DS's chip 0x160; Double Hero), the Double Attack of MegaMan and ProtoMan (docs/design/exe5ds-map.md §5,
-- §5.2: its controller the DS's effect object 0x97, its navis its T1 0x35). MegaMan fires seven volleys of his buster,
-- each a hit over the whole of the other side's area; ProtoMan swings his sword with each (his swings hit nothing);
-- both scatter flashes over the other side's panels. The chip's record and name are the DS's (written by
-- tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs); its record's use is the cannon's (action 0x14,
-- subtype 0: the DS copied Cannon's record), which nothing starts: no folder or hand holds it.

local attachment = require("../objects/attachment")
local buster = require("../lib/buster")
local cannon_chips = require("../chips/cannon")
local collision = require("../rules/collision")
local double_attack = require("../lib/double_attack")
local megaman = require("../navis/megaman").megaman
local panels = require("@exelib/panels")
local protoman = require("../navis/protoman").protoman
local regions = require("@exelib/regions")

type State = DoubleAttackNavi

-- The volleys (and swings): seven, 20 ticks apart; then 10 ticks.
local VOLLEYS, VOLLEY_TICKS, AFTER_TICKS = 7, 0x14, 0xA
local CHARGE_TICKS = 0x3C
-- MegaMan's shooting animation and ProtoMan's swing.
local ANIM_IDLE, ANIM_SHOOT, ANIM_SWING = 0, 0xE, 5
local SOUND_SHOT = asset.sound("buster-shot")
local SOUND_SWING = asset.sound("sword-swing")
-- ProtoMan's sword: the attachment table's row 3 (as his navi chip's), in its animation 1.
local SWORD = attachment.look { sprite = asset.sprite("sword"), palette = 0 }
local SWORD_ANIM = 1
-- The flashes each scatters (the effect table's row 0x3C, EXE5's, MegaMan's; row 0x16, the slash, ProtoMan's), three
-- a volley.
local FLASHES = {
    [0] = new.effect { sprite = asset.sprite("pink-flash"), anim = 0, palette = 2 },
    [1] = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 },
}
local FLASHES_A_VOLLEY = 3
-- A volley's hit (0x020A1018): the other side's area (region 0x82 for side 0, 0x81 for side 1), no spark, hitting navis
-- as a slash, hit modifier 3.
local HIT_MOD = 3

-- 0x020A1040: a flash on a panel of the other side's area but its own, at random (one `rng_positive` draw when there
-- is one), facing its way.
local function flash(me: Object)
    local list = double_attack.enemy_panels(me)
    if #list == 0 then
        return
    end
    local p = list[battle.rng_positive() % #list + 1]
    local cx, cy = field.center(panels.x(p), panels.y(p))
    local s = me.state :: State
    battle.effect(Vec3.new(cx, cy, 0), FLASHES[s.role], bit32.bxor(me.alliance, me.flip))
end

local function flashes(me: Object)
    for _ = 1, FLASHES_A_VOLLEY do
        flash(me)
    end
end

-- 0x020A1018
local function hit(me: Object)
    double_attack.hit(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = 0,
        z = 0,
        region = regions.other_area[me.alliance],
        target = collision.hits_navis,
        self_type = collision.slash,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
end

-- The end of a volley (or swing): the next, or after the seventh the action after.
local function next_volley(me: Object)
    if not double_attack.ran_out(me) then
        return
    end
    local n = me.timer2 - 1
    me.timer2 = n
    if n > 0 then
        me.phase_init = 0
        return
    end
    me:set_action(0xC)
end

-- 0x020A0CD4 (MegaMan), 0x020A0E94 (ProtoMan): the charge, then seven volleys; MegaMan lets go of both attachment
-- slots, ProtoMan of his first.
local function charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    local s = me.state :: State
    me.timer2 = VOLLEYS
    s.held = nil
    if s.role == 0 then
        s.held2 = nil
    end
    me:set_action(8)
end

-- 0x020A0D64: a volley: his buster arm and its muzzle flash, his overlay restarted (`sub_80C44D2`), the hit, three
-- flashes.
local function volley(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_SHOOT, 0xFF
        me.timer = VOLLEY_TICKS
        battle.play_sound(SOUND_SHOT)
        double_attack.detach(s.held)
        double_attack.raise_arm(me, "held")
        double_attack.detach(s.held2)
        attachment.attach(me, buster.flash, "held2", { anim = 0, while_dimmed = true })
        local overlay = me.related2
        if overlay then
            overlay.anim_loaded = 0xFF
        end
        hit(me)
        flashes(me)
        me.phase_init = 4
    end
    next_volley(me)
end

-- 0x020A0F20: a swing of his sword (no hit), three flashes.
local function swing(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_SWING, 0xFF
        double_attack.detach(s.held)
        attachment.attach(me, SWORD, "held", { anim = SWORD_ANIM, while_dimmed = true })
        battle.play_sound(SOUND_SWING)
        me.timer = VOLLEY_TICKS
        flashes(me)
        me.phase_init = 4
    end
    next_volley(me)
end

-- 0x020A0DDC (MegaMan: both slots let go), 0x020A0F80 (ProtoMan: his sword ends): 10 ticks, then he leaves.
local function after(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = ANIM_IDLE
        if s.role == 0 then
            s.held, s.held2 = nil, nil
        else
            double_attack.detach(s.held)
        end
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
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
    hit_param = 131,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "d-hero",
        damage = { 7, 10, 12, 15, 18, 20, 30, 35, 40, 40 },
        roles = {
            -- 0x020A0C70's table: MegaMan, (10, 10) from the user's spot.
            {
                navi = megaman,
                dx = 10,
                dy = 10,
                actions = {
                    [0x0] = function(me: Object)
                        double_attack.appear(me, 4, 0x10)
                    end,
                    [0x4] = charge,
                    [0x8] = volley,
                    [0xC] = after,
                    [0x10] = function(me: Object)
                        double_attack.leave(me, true)
                    end,
                },
            },
            -- 0x020A0E3C's: ProtoMan, (-10, -10).
            {
                navi = protoman,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = function(me: Object)
                        double_attack.appear(me, 4, 0x10)
                    end,
                    [0x4] = charge,
                    [0x8] = swing,
                    [0xC] = after,
                    [0x10] = function(me: Object)
                        double_attack.leave(me, false)
                    end,
                },
            },
        },
    },
}

return { ["d-hero"] = chip }
