--!strict
-- Count, the navi of the Count chips (the Japanese games' name; the US
-- release calls him HackJack; the original's actor object #0x11). The US
-- games have no routine for him: their navi chip table's entry 18
-- (`off_802CD5C[18]`) is null and the actor slot a placeholder. This is the Japanese games' code (EXE6
-- Falzar 0x080BCFCC, EXE6 Gregar 0x080BE82C; the two are the same but for
-- where things are), brought by the navi chip controller. He appears and
-- casts a rain over the middle of the other side's area (a dust storm,
-- lib/instant/dust_storm: the 3x3 around it, the whole area, hit every 12
-- ticks, five times, for the chip's damage), then drops up to three lances
-- (chips/count/lance), 10 ticks apart: first where an enemy stands in
-- that area, then on panels of it at random. Then he goes. The lances'
-- damage is the chip's parameter byte (50, 70, 100). See
-- docs/engine/chips.md §3.6.37.
--
-- Each chip's `navi` hook is `count.summon { lance_damage = n }`
-- (EXE6 Falzar 0x080BD236).

local dust_storm = require("../../lib/instant/dust_storm")
local navi_chips = require("../../lib/navi-chips/navi")
local panels = require("../../lib/panels")
local regions = require("../../lib/regions")
local lance = require("./lance")

local count = {}

-- What a chip summons him with.
export type Summon = {
    -- The lances' damage (the chip's first parameter byte, his Param1).
    lance_damage: number,
}

export type State = {
    -- The controller waiting for him (its flag's pointer sits in his
    -- collision data slot).
    controller: Object?,
    lance_damage: number,
    -- The lances' panels, packed x | y << 4 (ExtraVars[0..2]), how many
    -- (ExtraVars[0xC]) and the next (ExtraVars[0x10]).
    targets: { number },
    count: number,
    next: number,
}

local SPRITE = asset.sprite("count")
local SOUND_APPEAR, SOUND_RAIN = asset.sound("appear"), asset.sound("drip-shower")

-- The controller calls his spawner through r3, which his position takes
-- (EXE6 Falzar's address with the Thumb bit; EXE6 Gregar's is 0x080BEA97).
-- His init clears all of Z.
local SPAWNER_ADDRESS = 0x080BD237

-- By side: the middle of the other side's area, where the rain falls and
-- the lances' search is centred (two equal tables, at 0x080BD180 and
-- 0x080BD390 in EXE6 Falzar).
local MIDDLE = { [0] = { 5, 2 }, [1] = { 2, 2 } }

-- The rain: 60 ticks (the storm's Param2), untied (its Param3 2: it lands
-- its hits while dimmed); the storm's spawn Z, the register its spawner
-- leaves (its init keeps the fraction, none).
local RAIN: DustStorm = { ticks = 0x3C, tied = nil }
local RAIN_Z = 0x2_0000

-- The lances, as he drops them: at once, while dimmed (Param3 2). Their
-- spawn Z is the chip's damage flags (the register his spawner leaves).
local LANCE: CountLance = { while_dimmed = true, warn_ticks = 0 }

-- The lances' search, by side, in the 3x3 around the middle (region
-- 0x0F): an enemy's body on a solid panel of the other side's area, then
-- the rest of that area's solid panels (two tables of flag pairs).
local ENEMY_PANELS = { [0] = { 0x0400_0030, 0x3F40 }, [1] = { 0x0800_0010, 0x3F60 } }
local OTHER_PANELS = { [0] = { 0x30, 0x0400_3F40 }, [1] = { 0x10, 0x0800_3F60 } }
local MAX_LANCES = 3

-- How long the rain lasts, how often its sound comes; the lances' gap.
local RAIN_TICKS, RAIN_SOUND_TICKS = 0x46, 0x10
local LANCE_TICKS = 0xA

-- His actions.
local APPEAR, STAND, RAISE, RAIN_DOWN, LANCES, LOWER, LEAVE = 0x0, 0x4, 0x8, 0xC, 0x10, 0x14, 0x18

-- The init: on his panel, on the ground, his sprite with a shadow.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    me:set_lifecycle("update")
end

-- Enter an action's first phase: phase 4 with an animation and a timer;
-- `word` is the game's word store, which clears Timer2 too.
local function enter(me: Object, anim: number, ticks: number, word: boolean): boolean
    if me.phase ~= 0 then
        return false
    end
    me.phase = 4
    me.anim = anim
    me.timer = ticks
    if word then
        me.timer2 = 0
    end
    return true
end

-- Count the timer down; true at 0 or below.
local function ran_out(me: Object): boolean
    local left = me.timer - 1
    me.timer = left
    return left <= 0
end

-- He appears; 3 ticks; on to standing, or straight to leaving if his
-- panel can't hold him.
local function appear(me: Object)
    if enter(me, 3, 3, true) then
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
        return
    end
    if ran_out(me) then
        me:set_action(if navi_chips.has_footing(me) then STAND else LEAVE)
    end
end

-- 30 ticks standing.
local function stand(me: Object)
    if enter(me, 0, 0x1E, true) then
        return
    end
    if ran_out(me) then
        me:set_action(RAISE)
    end
end

-- 20 ticks raising his arms (animation 7 at 17 left).
local function raise(me: Object)
    if enter(me, 5, 0x14, false) then
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x11 then
        me.anim = 7
    elseif left <= 0 then
        me:set_action(RAIN_DOWN)
    end
end

-- The rain over the middle of the other side's area (the storm's spawner,
-- `sub_80C6330`, with his damage word), its sound now and every 16 ticks;
-- 70 ticks.
local function rain_down(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local at = MIDDLE[me.alliance]
        local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
        dust_storm.spawn(me, at[1], at[2], RAIN_Z, me.element, damage, RAIN)
        battle.play_sound(SOUND_RAIN)
        me.timer = RAIN_TICKS
        me.timer2 = RAIN_SOUND_TICKS
        return
    end
    local next = me.timer2 - 1
    me.timer2 = next
    if next <= 0 then
        battle.play_sound(SOUND_RAIN)
        me.timer2 = RAIN_SOUND_TICKS
    end
    if ran_out(me) then
        me:set_action(LANCES)
    end
end

-- The lances' panels (up to three): around the middle of the other side's
-- area, the panels an enemy stands on; if fewer than three, the area's
-- other panels shuffled after them (`sub_8000C72`, as many swaps as
-- panels); with three or more enemy panels, or none else, those shuffled.
local function search(me: Object): { number }
    local at = MIDDLE[me.alliance]
    local dir = panels.side_direction(me.alliance)
    local first = ENEMY_PANELS[me.alliance]
    local found = panels.region(at[1], at[2], regions.square, dir, first[1], first[2])
    local n = #found
    if n < MAX_LANCES then
        local second = OTHER_PANELS[me.alliance]
        local rest = panels.region(at[1], at[2], regions.square, dir, second[1], second[2])
        if #rest > 0 then
            panels.shuffle(rest, #rest, #rest)
            for _, p in rest do
                table.insert(found, p)
            end
            return found
        elseif n == 0 then
            return found
        end
    end
    panels.shuffle(found, n, n)
    return found
end

-- A lance on the packed panel `p`: the chip's lance damage with his
-- damage word's flags (the double-damage, paralysis, uninstall and erase
-- bits), no hit parameter; its Z the flags too (the register left).
local function drop_lance(me: Object, p: number)
    local s = me.state :: State
    local flags = bit32.band(me.damage, 0xF000)
    lance.spawn(me, panels.x(p), panels.y(p), me.element, bit32.bor(s.lance_damage, flags), flags, LANCE)
end

-- The lances: their panels (none: on to lowering), then one every 10
-- ticks.
local function lances(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        local found = search(me)
        local count = math.min(#found, MAX_LANCES)
        for i = 1, count do
            s.targets[i] = found[i]
        end
        s.count = count
        if count == 0 then
            me:set_action(LOWER)
            return
        end
        s.next = 0
        me.timer = LANCE_TICKS
        return
    end
    if not ran_out(me) then
        return
    end
    drop_lance(me, s.targets[s.next + 1])
    me.timer = LANCE_TICKS
    s.next += 1
    if s.next < s.count then
        return
    end
    me:set_action(LOWER)
end

-- 23 ticks lowering his arms (animation 0 at 20 left).
local function lower(me: Object)
    if enter(me, 8, 0x17, true) then
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left == 0x14 then
        me.anim = 0
    elseif left <= 0 then
        me:set_action(LEAVE)
    end
end

-- 3 ticks (to -1), then he goes and the controller moves on.
local function leave(me: Object)
    if enter(me, 4, 3, true) then
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS: { [number]: (me: Object) -> () } = {
    [APPEAR] = appear,
    [STAND] = stand,
    [RAISE] = raise,
    [RAIN_DOWN] = rain_down,
    [LANCES] = lances,
    [LOWER] = lower,
    [LEAVE] = leave,
}

count.kind = define.kind {
    id = "count/navi",
    pool = "actor",
    state = { controller = "object", lance_damage = "u8", targets = "u8[3]", count = "u8", next = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            local action = ACTIONS[me.action]
            if not action then
                error(string.format("Count: action %#x reads past his table", me.action))
            end
            action(me)
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- A chip's `navi` hook: Count for the user, on the panel the
-- controller picked, reporting to `controller`.
function count.summon(args: Summon): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(count.kind, user, spec, SPAWNER_ADDRESS)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.lance_damage = args.lance_damage
        return o
    end
end

return count
