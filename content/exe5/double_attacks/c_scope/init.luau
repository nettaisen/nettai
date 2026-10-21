--!strict
-- C-SCOPE (the DS's chip 0x164; Cross Scope), the Double Attack of MegaMan and SearchMan (docs/design/exe5ds-map.md §5,
-- §5.2: its director the DS's effect object 0x9B, its actors its T1 0x3A, its scopes its effect object 0xA4). As
-- SearchMan's charge ends, two scopes (./scope) come three columns ahead and walk crossing diagonals, until the player
-- stops them with A (the side's navi's A pressed while dimmed: its AIData +0x2C bit 1, 0x02015914) or they have
-- walked eight rounds; then each actor fires five times on its scope's panel, one every six ticks (a hit reaching
-- invisible targets: a curse). MegaMan waits for SearchMan's word. Both wait for the other to be done before they
-- leave. The chip's record and name are the DS's (written by tools/exe5/gen_content.py, bn6battle-verify, from the DS
-- ROMs); its record's use is the cannon's (action 0x14, subtype 0: the DS copied Cannon's record), which nothing
-- starts: no folder or hand holds it.

local attachment = require("../objects/attachment")
local buster = require("../lib/buster")
local cannon_chips = require("../chips/cannon")
local collision = require("../rules/collision")
local double_attack = require("../lib/double_attack")
local megaman = require("../navis/megaman").megaman
local regions = require("@exelib/regions")
local scope = require("@self/scope")
local searchman = require("../navis/searchman").searchman
local sparks = require("../lib/sparks")

-- What each actor keeps besides the shared fields: SearchMan's word that the shots start (the DS's Param4, +7, which
-- he sets on both), the two scopes (+0x74, +0x78: SearchMan's, copied to MegaMan as they come).
type State = DoubleAttackActor & {
    fire: boolean,
    scope: Object?,
    scope2: Object?,
}

local CHARGE_TICKS = 0x3C
-- The shots: five each, six ticks apart (`bge`); then 30 ticks.
local SHOTS, SHOT_TICKS, AFTER_TICKS = 5, 5, 0x1E
-- Where the scopes come: three columns ahead, centered.
local SCOPES_AHEAD = 3
local ANIM_IDLE, ANIM_SHOOT, ANIM_SEARCH = 0, 0xE, 0x11
local SOUND_SHOT = asset.sound("col-army-2")
local SOUND_LOCK = asset.sound("mark")
-- A shot's hit (0x02015A48): the scope's panel (region 1), spark 0, hitting navis as a curse, hit modifier 3.
local HIT_MOD = 3

-- 0x02015A48: a shot on the scope's panel.
local function shoot_at(me: Object, target: Object?)
    if not target then
        return
    end
    double_attack.hit(me, {
        panel_x = target.panel_x,
        panel_y = target.panel_y,
        element = 0,
        z = 0,
        region = regions.single,
        hit_effect = sparks.plain,
        target = collision.hits_navis,
        self_type = collision.curse,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
end

-- A shot's end: the next, or after the fifth the action after.
local function next_shot(me: Object)
    local t = me.timer - 1
    me.timer = t
    if t >= 0 then
        return
    end
    me.phase_init = 0
    local n = me.timer2 - 1
    me.timer2 = n
    if n <= 0 then
        me:set_action(0xC)
    end
end

-- The actors' action 0xC (0x0201571C, 0x0201599C): once the other is at it too, 30 ticks, then he leaves; MegaMan lets
-- go of his attachments, SearchMan ends the scopes.
local function after(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if double_attack.partner(me).action ~= 0xC then
            return
        end
        me.anim = ANIM_IDLE
        if s.role == 0 then
            s.held, s.held2 = nil, nil
        else
            scope.finish(s.scope)
            scope.finish(s.scope2)
        end
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
    end
end

-- MegaMan's charge (0x020155C0): then his future panel the one ahead in row 1 (nothing reads it), no word yet, and his
-- shots.
local function megaman_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me.future_panel_x, me.future_panel_y = me.panel_x + me.facing, 1
    ;(me.state :: State).fire = false
    me.timer2 = 2
    me:set_action(8)
end

-- MegaMan's shots (0x0201565C's phases): his buster up, he waits for SearchMan's word; then five shots on the first
-- scope's panel, his arm and its muzzle flash each time (none without a scope: he is done).
local function megaman_shots(me: Object)
    local s = me.state :: State
    local p = me.phase
    if p == 0 then
        -- 0x0201566C
        if me.phase_init == 0 then
            me.anim, me.anim_loaded = ANIM_SHOOT, 0xFF
            double_attack.detach(s.held)
            double_attack.megaman_arm(me, "held")
            me.phase_init = 4
        end
        if not s.fire then
            return
        end
        if not s.scope then
            me:set_action(0xC)
            return
        end
        me.timer2 = SHOTS
        me.phase, me.phase_init = 4, 0
    elseif p == 4 then
        -- 0x020156B0
        if me.phase_init == 0 then
            me.anim, me.anim_loaded = ANIM_SHOOT, 0xFF
            double_attack.detach(s.held)
            double_attack.megaman_arm(me, "held")
            double_attack.detach(s.held2)
            attachment.attach(me, buster.flash, "held2", { anim = 0, while_dimmed = true })
            me.timer = SHOT_TICKS
            shoot_at(me, s.scope)
            battle.play_sound(SOUND_SHOT)
            me.phase_init = 4
        end
        next_shot(me)
    else
        error(string.format("C-SCOPE's MegaMan: phase %#x reads past his table (0x020D803C)", p))
    end
end

-- SearchMan's charge (0x020157F0): then, three columns ahead on the field, the scopes one ahead and one behind of it in
-- the row above, the second walking the other way; MegaMan gets both; no word yet.
local function searchman_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    local s = me.state :: State
    s.scope, s.scope2 = nil, nil
    local f = me.facing
    local x, y = me.panel_x + SCOPES_AHEAD * f, me.panel_y
    if field.valid(x, y) then
        s.scope = scope.spawn(me, x - f, y - 1, 0)
        s.scope2 = scope.spawn(me, x + f, y - 1, 1)
    end
    local t = double_attack.partner(me).state :: State
    t.scope, t.scope2 = s.scope, s.scope2
    s.fire = false
    me:set_action(8)
end

-- SearchMan's shots (0x020158D4's phases): his scope up; he waits while the first scope walks and the player holds
-- off A (the side's navi's press while dimmed); then the word to both, the scopes stop, and five shots on the second
-- scope's panel (none without one: he is done).
local function searchman_shots(me: Object)
    local s = me.state :: State
    local p = me.phase
    if p == 0 then
        -- 0x020158E4
        if me.phase_init == 0 then
            me.anim, me.anim_loaded = ANIM_SEARCH, 0xFF
            me.phase_init = 4
        end
        if s.scope and not scope.paused(s.scope) then
            -- sub_80103BC: the side's navi.
            local navi = battle.player(me.alliance)
            if navi and not navi:dimmed_pressed("a") then
                return
            end
        end
        ;(double_attack.partner(me).state :: State).fire = true
        s.fire = true
        scope.stop(s.scope)
        scope.stop(s.scope2)
        if not s.scope2 then
            me:set_action(0xC)
            return
        end
        me.timer2 = SHOTS
        battle.play_sound(SOUND_LOCK)
        me.phase, me.phase_init = 4, 0
    elseif p == 4 then
        -- 0x02015950
        if me.phase_init == 0 then
            me.anim, me.anim_loaded = ANIM_SEARCH, 0xFF
            me.timer = SHOT_TICKS
            shoot_at(me, s.scope2)
            battle.play_sound(SOUND_SHOT)
            me.phase_init = 4
        end
        next_shot(me)
    else
        error(string.format("C-SCOPE's SearchMan: phase %#x reads past his table (0x020D8058)", p))
    end
end

-- SearchMan appears wearing what he wears (0x02015794: his NameID's actor record a player's, `sub_8011420` with
-- Param3 1).
local function searchman_appear(me: Object)
    double_attack.appear(me, 4, 0x10, true)
end

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "special",
    class = "special",
    rarity = 0,
    mb = 5,
    damage = 40,
    hit_param = 133,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "c-scope",
        damage = { 15, 15, 15, 15, 15, 22, 30, 35, 40, 40 },
        state = { fire = "bool", scope = "object", scope2 = "object" },
        roles = {
            -- 0x0201555C's table: MegaMan, (10, 10) from the user's spot.
            {
                navi = megaman,
                dx = 10,
                dy = 10,
                actions = {
                    [0x0] = function(me: Object)
                        double_attack.appear(me, 4, 0x10)
                    end,
                    [0x4] = megaman_charge,
                    [0x8] = megaman_shots,
                    [0xC] = after,
                    [0x10] = function(me: Object)
                        double_attack.leave(me, true)
                    end,
                },
            },
            -- 0x02015784's: SearchMan, (-10, -10).
            {
                navi = searchman,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = searchman_appear,
                    [0x4] = searchman_charge,
                    [0x8] = searchman_shots,
                    [0xC] = after,
                    [0x10] = function(me: Object)
                        double_attack.leave(me, true)
                    end,
                },
            },
        },
    },
}

return { ["c-scope"] = chip }
