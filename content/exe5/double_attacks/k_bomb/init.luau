--!strict
-- K-BOMB (the DS's chip 0x16A; Kokuban-keshi Bomb), the Double Attack of MegaMan and NumberMan
-- (docs/design/exe5ds-map.md §5, §5.2: its controller the DS's effect object 0xA1, its navis its T1 0x42, the bombs its
-- attack object 0xCC). Each throws two bombs (./bomb), 31 ticks apart, lobbed at the panel three columns ahead; each
-- bursts into four hits on a quarter of the 3x3 around it (MegaMan's the first and third quarters, NumberMan's the
-- second and fourth), its middle panel in every quarter, confusing. The chip's record and name are the DS's (written by
-- tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs); its record's use is the cannon's (action 0x14,
-- subtype 0: the DS copied Cannon's record), which nothing starts: no folder or hand holds it.

local attachment = require("../objects/attachment")
local bomb = require("@self/bomb")
local bombs = require("../lib/bombs")
local cannon_chips = require("../chips/cannon")
local double_attack = require("../lib/double_attack")
local megaman = require("../navis/megaman").megaman
local numberman = require("../navis/numberman").numberman

type State = DoubleAttackNavi

local CHARGE_TICKS = 0x3C
-- The throws: two, 31 ticks each (the bomb out on the tenth); NumberMan waits 15 ticks first. Then 30 ticks.
local THROWS, THROW_TICKS, RELEASE_AT, WAIT_TICKS, AFTER_TICKS = 2, 0x1E, 9, 0xF, 0x1E
local ANIM_THROW = 6
local SOUND_THROW = asset.sound("minibomb-throw")
-- Where the bomb starts: 4 pixels ahead and 48 up.
local BOMB_X, BOMB_Z = 0x4_0000, 0x30_0000

-- 0x0201611C: a bomb at its hand, on quarter `first` on the first throw, `later` on the second.
local function release(me: Object, first: number, later: number)
    local s = me.state :: State
    s.held = nil
    local p = me.pos
    local quarter = if me.timer2 == THROWS then first else later
    bomb.spawn(me, Vec3.new(int.i32(p.x + me.facing * BOMB_X), p.y, int.i32(p.z + BOMB_Z)), quarter)
end

-- A throw (0x020A1444, MegaMan's; 0x020A1650, NumberMan's): the bomb in his hand (the attachment table's row 4),
-- thrown on the tenth tick; 31 ticks; then the next, or after the second the action after. MegaMan's overlay restarts
-- (`sub_80C44D2`).
local function throw(me: Object, first: number, later: number)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim, me.anim_loaded = ANIM_THROW, 0xFF
        attachment.attach(me, bombs.held, "held", { anim = 0, while_dimmed = true })
        me.timer = 0
        if s.role == 0 then
            local overlay = me.related2
            if overlay then
                overlay.anim_loaded = 0xFF
            end
        end
        battle.play_sound(SOUND_THROW)
        me.phase_init = 4
    end
    if me.timer == RELEASE_AT then
        release(me, first, later)
    end
    local t = me.timer + 1
    me.timer = t
    if t <= THROW_TICKS then
        return
    end
    me.timer = 0
    local n = me.timer2 - 1
    me.timer2 = n
    if n > 0 then
        me.phase_init = 0
        return
    end
    me:set_action(0xC)
end

-- MegaMan's charge (0x020A13AC): then his future panel the one ahead in row 1 (nothing reads it), and two throws.
local function megaman_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    me.timer2 = THROWS
    me:set_action(8)
end

-- NumberMan's attack (0x020A161C's phases): 15 ticks, then two throws.
local function numberman_attack(me: Object)
    local p = me.phase
    if p == 0 then
        -- 0x020A162C
        if me.phase_init == 0 then
            me.timer = WAIT_TICKS
            me.phase_init = 4
        end
        if double_attack.ran_out(me) then
            me.timer2 = THROWS
            me.phase, me.phase_init = 4, 0
        end
    elseif p == 4 then
        throw(me, 1, 3)
    else
        error(string.format("K-BOMB's NumberMan: phase %#x reads past his table (0x02135B1C)", p))
    end
end

-- Action 0xC of both (0x020A14D4, 0x020A16D8): 30 ticks, then he leaves.
local function after(me: Object)
    if me.phase_init == 0 then
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
    end
end

-- NumberMan appears wearing what he wears (0x020A153C: his NameID's actor record a player's, `sub_8011420` with Param3
-- 1).
local function numberman_appear(me: Object)
    double_attack.appear(me, 4, 0x10, true)
end

local function leave(me: Object)
    double_attack.leave(me, true)
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 148,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "k-bomb",
        damage = { 40, 40, 40, 40, 40, 50, 70, 90, 100, 100 },
        roles = {
            -- 0x02135AEC's table: MegaMan, (10, 10) from the user's spot.
            {
                navi = megaman,
                dx = 10,
                dy = 10,
                actions = {
                    [0x0] = function(me: Object)
                        double_attack.appear(me, 4, 0x10)
                    end,
                    [0x4] = megaman_charge,
                    [0x8] = function(me: Object)
                        throw(me, 0, 2)
                    end,
                    [0xC] = after,
                    [0x10] = leave,
                },
            },
            -- NumberMan, (-10, -10).
            {
                navi = numberman,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = numberman_appear,
                    [0x4] = function(me: Object)
                        if double_attack.charge(me, CHARGE_TICKS) then
                            me:set_action(8)
                        end
                    end,
                    [0x8] = numberman_attack,
                    [0xC] = after,
                    [0x10] = leave,
                },
            },
        },
    },
}

return { ["k-bomb"] = chip }
