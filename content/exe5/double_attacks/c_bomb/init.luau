--!strict
-- C-BOMB (the DS's chip 0x165; Chemical Bomb), the Double Attack of NapalmMan and Meddy (docs/design/exe5ds-map.md §5,
-- §5.2: its director the DS's effect object 0x9C, its actors its T1 0x3B, Meddy's capsule its attack object 0xCD).
-- Meddy puts a capsule (./capsule) on a panel of the other side's at random as she winds up; if she did, NapalmMan,
-- charged, marks the panel four columns ahead and lobs his chip's napalm bomb there (chips/napalmmn/bomb: its fire
-- spreads over the 3x3 around an empty panel). Anything that touches the capsule while it lies there (the napalm fire
-- among them) sets it off 30 ticks later over the 3x3 around it, paralyzing. Without a capsule neither attacks. Both
-- wait for the other to be done before they leave. The chip's record and name are the DS's (written by
-- tools/exe5/gen_content.py, bn6battle-verify, from the DS ROMs); its record's use is the cannon's (action 0x14,
-- subtype 0: the DS copied Cannon's record), which nothing starts: no folder or hand holds it.

local cannon_chips = require("../chips/cannon")
local capsule = require("@self/capsule")
local double_attack = require("../lib/double_attack")
local meddy = require("../navis/meddy").meddy
local napalm_bomb = require("../chips/napalmmn/bomb")
local napalmman = require("../navis/napalmman").napalmman
local panels = require("@exelib/panels")

-- What each actor keeps besides the shared fields: Meddy's capsule (+0x74), NapalmMan's bomb (+0x78).
type State = DoubleAttackActor & {
    capsule: Object?,
    bomb: Object?,
}

local CHARGE_TICKS = 0x3C
-- NapalmMan: his aim (animation 18, 31 ticks), his throw (animation 19, 89 ticks), his wait after the bomb (31 ticks);
-- the bomb four columns ahead, from 8 pixels ahead and 56 up (the napalm bomb's variant 1: its fall, no lit target).
local ANIM_IDLE, ANIM_AIM, ANIM_THROW = 0, 0x12, 0x13
local AIM_TICKS, THROW_TICKS, AFTER_TICKS = 0x1E, 0x58, 0x1E
local REACH = 4
local MUZZLE_X, MUZZLE_Z = 0x8_0000, 0x38_0000
local GRAVITY = -0xA000
-- The mark on his target (the DS's effect row 0x4C, the GBA's too: 10-1C), for 30 ticks.
local MARK = new.effect { sprite = asset.sprite("target-mark"), anim = 0, palette = 0 }
local MARK_TICKS = 0x1E
local SOUND_MARK = asset.sound("mark")
-- Meddy: her wind-up (animation 16, 60 ticks), the capsule out 20 ticks in (animation 18).
local ANIM_WIND_UP, ANIM_PLACE = 0x10, 0x12
local PLACE_AT = 0x28
-- The panels the capsule may go on (0x020D80B0, by side): the other side's, solid, with no one on them.
local CAPSULE_PANELS = { [0] = { 0x30, 0x0F800000 }, [1] = { 0x10, 0x0F800020 } }

-- The other actor (the DS's +0x60, which the director links as it brings both).
local function partner(me: Object): Object
    local s = me.state :: State
    for _, o in battle.objects_of(me.kind) do
        if o ~= me and (o.state :: State).director == s.director then
            return o
        end
    end
    error("C-BOMB's actor has no partner (its +0x60 reads through a null pointer)")
end

-- A team navi appears wearing what it wears (0x02015B0C, 0x02015D58: its NameID's actor record a player's,
-- `sub_8011420` with Param3 1).
local function appear_wearing(me: Object)
    local first = me.phase_init == 0
    double_attack.appear(me, 4, 0x10)
    if first then
        me:add_parts(me.identity :: Identity, 1)
    end
end

-- Action 0xC of both (0x02015CE0, 0x02015E0C): once the other is at it too, 30 ticks, then they leave. NapalmMan lets
-- go of his attachment slots; Meddy forgets her capsule, which then goes in a puff (if it still lies there).
local function after(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        if partner(me).action ~= 0xC then
            return
        end
        if s.role == 0 then
            me.anim = ANIM_IDLE
            s.held, s.held2 = nil, nil
        else
            s.capsule = nil
            me.anim = ANIM_IDLE
        end
        me.timer = AFTER_TICKS
        me.phase_init = 4
    end
    if double_attack.ran_out(me) then
        me:set_action(0x10)
    end
end

-- NapalmMan's charge (0x02015B68): then his attack if Meddy put a capsule out, else he is done.
local function napalmman_charge(me: Object)
    if not double_attack.charge(me, CHARGE_TICKS) then
        return
    end
    me:set_action(if (partner(me).state :: State).capsule then 8 else 0xC)
end

-- The bomb lies there yet (its thrower's slot, +0x78, which the bomb empties as it ends: 0x0220A196, 0x0220A220,
-- 0x0220A338, as it goes to its destroy state; here read off the bomb).
local function bomb_lies(s: State): boolean
    local b = s.bomb
    return b ~= nil and b.kind == napalm_bomb.kind and b.lifecycle ~= "destroy"
end

-- NapalmMan's attack (0x02015BF8's phases).
local function napalmman_attack(me: Object)
    local s = me.state :: State
    local p = me.phase
    local f = me.facing
    if p == 0 then
        -- 0x02015C08: his cannon up, his target marked.
        if me.phase_init == 0 then
            me.anim = ANIM_AIM
            me.timer = AIM_TICKS
            local x, y = field.center(me.panel_x + REACH * f, me.panel_y)
            local mark = battle.effect(Vec3.new(x, y, 0), MARK)
            if mark then
                mark.timer = MARK_TICKS
            end
            battle.play_sound(SOUND_MARK)
            me.phase_init = 4
            return
        end
        if double_attack.ran_out(me) then
            me.phase, me.phase_init = 4, 0
        end
    elseif p == 4 then
        -- 0x02015C54: the bomb (exe5_80CC71A, running while dimmed) at the panel four columns ahead (the field's edge
        -- column at most).
        if me.phase_init == 0 then
            me.anim = ANIM_THROW
            local tx = math.clamp(me.panel_x + REACH * f, 0, 7)
            local pos = me.pos
            s.bomb = napalm_bomb.lob(me, Vec3.new(int.i32(pos.x + f * MUZZLE_X), pos.y, int.i32(pos.z + MUZZLE_Z)), {
                gravity = GRAVITY,
                target_x = tx,
                target_y = me.panel_y,
                damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)),
                while_dimmed = true,
            })
            me.timer = THROW_TICKS
            me.phase_init = 4
            return
        end
        if double_attack.ran_out(me) then
            me.phase, me.phase_init = 8, 0
        end
    elseif p == 8 then
        -- 0x02015CB8: once the bomb is gone, 30 ticks.
        if me.phase_init == 0 then
            if bomb_lies(s) then
                return
            end
            me.timer = AFTER_TICKS
            me.phase_init = 4
        end
        if double_attack.ran_out(me) then
            me:set_action(0xC)
        end
    else
        error(string.format("C-BOMB's NapalmMan: phase %#x reads past his table (0x020D8090)", p))
    end
end

-- 0x02015EB0: a panel for the capsule: from her column on forward while the column is on the field, its panels the
-- other side's, solid and empty, but NapalmMan's target, one at random (an `rng_positive` draw when there is one).
local function capsule_panel(me: Object): (number?, number?)
    local f = me.facing
    local cond = CAPSULE_PANELS[me.alliance]
    local list: { number } = {}
    local x = me.panel_x
    while field.valid(x, 1) do
        for y = 1, 3 do
            if field.check(x, y, cond[1], cond[2]) and not (x == me.panel_x + REACH * f and y == me.panel_y) then
                local packed: number = panels.pack(x, y)
                table.insert(list, packed)
            end
        end
        x += f
    end
    if #list == 0 then
        return nil, nil
    end
    local p = list[battle.rng_positive() % #list + 1]
    return panels.x(p), panels.y(p)
end

-- Meddy's wind-up (0x02015DB4): 60 ticks; 20 in, a capsule on a panel of the other side's; then her action 8 (at once
-- 0xC: 0x02015E00) with a capsule out, else 0xC.
local function meddy_place(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.anim = ANIM_WIND_UP
        me.timer = CHARGE_TICKS
        me.phase_init = 4
    end
    if me.timer == PLACE_AT then
        me.anim = ANIM_PLACE
        local x, y = capsule_panel(me)
        if x and y then
            s.capsule = capsule.spawn(me, x, y)
        end
    end
    if double_attack.ran_out(me) then
        me:set_action(if s.capsule then 8 else 0xC)
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
    hit_param = 158,
    lockout = 0,
    flags = {},
    action = cannon_chips.cannon.action,
    double_attack = double_attack.make {
        id = "c-bomb",
        damage = { 75, 75, 75, 75, 75, 75, 110, 130, 150, 150 },
        state = { capsule = "object", bomb = "object" },
        roles = {
            -- 0x02015AFC's table: NapalmMan, (10, 10) from the user's spot.
            {
                navi = napalmman,
                dx = 10,
                dy = 10,
                actions = {
                    [0x0] = appear_wearing,
                    [0x4] = napalmman_charge,
                    [0x8] = napalmman_attack,
                    [0xC] = after,
                    [0x10] = function(me: Object)
                        double_attack.leave(me, true)
                    end,
                },
            },
            -- 0x02015D48's: Meddy, (-10, -10).
            {
                navi = meddy,
                dx = -10,
                dy = -10,
                actions = {
                    [0x0] = appear_wearing,
                    [0x4] = meddy_place,
                    [0x8] = function(me: Object)
                        me:set_action(0xC)
                    end,
                    [0xC] = after,
                    [0x10] = function(me: Object)
                        double_attack.leave(me, true)
                    end,
                },
            },
        },
    },
}

return { ["c-bomb"] = chip }
