--!strict
-- GyroMan, the navi of EXE5's GyroMan chips (EXE5's actor object #0x0C, 0x080BAA80; EXE5's own, built on EXE6's navi
-- template), brought by their navi chip controller, wearing his propeller (EXE5's body overlay row 0). He appears,
-- stands 40 ticks, rises and vanishes, and takes off from his side's back column along his row, a pixel up: 16
-- ticks on (a sound) he flies forward 3 pixels a tick, dropping a bomb (chips/gyroman/bomb) on each enemy panel as
-- he passes its center, until he reaches the field's edge; then he leaves, and the controller moves on. Holding L
-- and R as he takes off (a sound) sends him on a bombing run instead: he flies on to the first panel holding an
-- enemy navi, stops over it and drops a bomb every 16 ticks, one for each enemy panel in his row, and 16 ticks on
-- flies off the field.
--
-- GyroMan the navi's own chip, Airforce (navis/gyroman/chip), flies the same object in his place (its fourth
-- parameter 1, by another spawner, 0x080BAE8A): it starts at the take-off, with no bombing run; it stands still
-- while the battle is dimmed; its bombs are the variant that waits while dimmed too; its propeller's sprite doesn't
-- step while paused; and it reports to no controller: the navi watches for it to go.

local navi_chips = require("@exelib/navi_chips/navi")
local bomb = require("./bomb")

export type State = {
    -- The controller waiting for him (the flag it gave him, +0x54).
    controller: Object?,
    -- The chip's level (its Param1: 0, 3 for SP, 4 for DS), which picks his palette.
    level: number,
    -- L and R were held as he took off (+5): the bombing run.
    run: boolean,
    -- The bombs left to drop on his run (+6).
    bombs: number,
    -- Its fourth parameter (+7): GyroMan the navi's own flight (Airforce).
    airforce: boolean,
}

local gyroman = {}

local SPRITE = asset.sprite("gyroman")
local SOUND_APPEAR = asset.sound("appear")
-- (0x104 and 0x105: 0x080BAC56 and 0x080BACBE load them as 5 + 255 and 6 + 255.)
local SOUND_TAKEOFF = asset.sound("gyro-takeoff")
local SOUND_DROP = asset.sound("drop")
local SOUND_RUN = asset.sound("ok-8b")
-- What he wears (0x0800EE4E: EXE5's body overlay row 0, its sprite 10-0A, behind him in his animations 0 to 5, 7,
-- 16 and 17), stepping even while paused.
local PROPELLER = define.identity {
    id = "gyroman-navi",
    class = "navi",
    parts = {
        body = {
            sprite = asset.sprite("gyro-propeller"),
            anims = 22,
            behind = { { 0, 5 }, { 7, 7 }, { 16, 17 } },
        },
    },
}
-- By the level (0x0800DA8A: navi 2's count, 1, by 0x0800DAE8's row): SP's 2, DS's 3.
local PALETTES: { [number]: number } = { [0] = 0, 0, 0, 2, 3 }
-- An enemy panel, by side (the panel flag 0x20, set or clear: 0x080BACEC, 0x080BAEE4).
local ENEMY_AREA = { [0] = 0x20, 0 }
-- A panel holding an enemy navi's body, by side (0x080BAD74).
local ENEMY_NAVI = { [0] = 0x0400_0000, 0x0800_0000 }
-- His speed (16.16 a tick, forward), and the ticks between bombs on his run.
local SPEED, BETWEEN = 0x3_0000, 0xF
-- His bombs' variant (his +7 and 1): the chips' 1, Airforce's 2.
local BOMB_VARIANT, AIRFORCE_BOMB_VARIANT = 1, 2

local function damage_word(me: Object): number
    return bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
end

-- A bomb on his panel (a sound).
local function drop(me: Object)
    battle.play_sound(SOUND_DROP)
    local variant = if (me.state :: State).airforce then AIRFORCE_BOMB_VARIANT else BOMB_VARIANT
    bomb.drop(me, me.panel_x, me.panel_y, me.element, variant, damage_word(me))
end

-- The flight: forward one tick; whether he passed his panel's center (from off it) this tick.
local function fly(me: Object): boolean
    local cx = field.center(me.panel_x, me.panel_y)
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    me.pos = Vec3.new(x, p.y, p.z)
    return cx ~= p.x and navi_chips.between(x, p.x, cx)
end

-- At the field's edge column.
local function at_edge(me: Object): boolean
    return me.panel_x == 0 or me.panel_x == 7
end

-- 0x080BAEF4: as he takes off, L and R held (by his user, +0x4C) start his bombing run, with a sound. (Not
-- Airforce's flight.)
local function check_run(me: Object)
    local s = me.state :: State
    if s.airforce or s.run then
        return
    end
    local user = me.related1 :: Object
    if user:held("l") and user:held("r") then
        s.run = true
        battle.play_sound(SOUND_RUN)
    end
end

-- 0x080BAEB2: the enemy panels of row `y` (columns 1 to 6).
local function enemy_panels(me: Object, y: number): number
    local set, clear = ENEMY_AREA[me.alliance], bit32.bxor(ENEMY_AREA[me.alliance], 0x20)
    local n = 0
    for x = 1, 6 do
        if field.check(x, y, set, clear) then
            n += 1
        end
    end
    return n
end

-- Action 0 (0x080BAB50): he appears (animation 3, a sound); 4 ticks.
local function appear(me: Object)
    if me.phase_init == 0 then
        me.anim = 3
        battle.play_sound(SOUND_APPEAR)
        me.visible = true
    end
    if navi_chips.wait_on_init(me, 4, true) then
        me:set_action(4)
    end
end

-- Action 4 (0x080BAB86): 40 ticks standing.
local function stand(me: Object)
    if me.phase_init == 0 then
        me.anim = 0
    end
    if navi_chips.wait_on_init(me, 0x28, true) then
        me:set_action(8)
    end
end

-- Action 8 (0x080BABAE): he rises (animation 4), 30 ticks (a halfword store), vanishing 4 in.
local function rise(me: Object)
    if me.phase_init == 0 then
        me.anim = 4
    end
    local left = navi_chips.countdown_on_init(me, 0x1E)
    if left == nil then
        return
    end
    if left == 0x1A then
        me.visible = false
    elseif left <= 0 then
        me:set_action(0xC)
    end
end

-- Action 0xC (0x080BABE6): at his side's back column (animation 20), 4 pixels further back and a pixel up, shown,
-- his speed set; 16 ticks, watching for L and R; then (a sound) off he flies: on his run if they were held.
local function take_off(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = 0x14
        me.panel_x = if bit32.bxor(me.alliance, me.flip) ~= 0 then 6 else 1
        me:set_coordinates_from_panel()
        local p = me.pos
        me.pos = Vec3.new(int.i32(p.x - me.facing * 0x4_0000), int.i32(p.y + 0x1_0000), int.i32(p.z - 0x1_0000))
        me.visible = true
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        me.timer, me.timer2 = 0xF, 0
    end
    check_run(me)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    battle.play_sound(SOUND_TAKEOFF)
    me:set_action(if s.run and not s.airforce then 0x14 else 0x10)
end

-- Action 0x10 (0x080BAC7A): he flies, dropping a bomb on each enemy panel whose center he passes, to the edge.
local function bomb_row(me: Object)
    if fly(me) then
        if at_edge(me) then
            me:set_action(0x18)
            return
        end
        if bit32.band(field.flags(me.panel_x, me.panel_y), 0x20) == ENEMY_AREA[me.alliance] then
            drop(me)
        end
    end
    me:set_panel_from_coordinates()
end

-- Run phase 0 (0x080BAD14): he flies on to the first panel holding an enemy navi, and stops over it; or to the edge.
local function seek(me: Object)
    if fly(me) then
        if at_edge(me) then
            me:set_action(0x18)
            return
        end
        if bit32.band(field.flags(me.panel_x, me.panel_y), ENEMY_NAVI[me.alliance]) ~= 0 then
            me.phase, me.phase_init = 4, 0
            return
        end
    end
    me:set_panel_from_coordinates()
end

-- Run phase 4 (0x080BAD7C): a bomb at once and every 16 ticks, one for each enemy panel in his row.
local function bombard(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        me.phase_init = 4
        s.bombs = enemy_panels(me, me.panel_y)
        if s.bombs == 0 then
            me.phase, me.phase_init = 8, 0
            return
        end
    else
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
    end
    drop(me)
    me.timer, me.timer2 = BETWEEN, 0
    local left = s.bombs - 1
    s.bombs = int.u8(left)
    if left <= 0 then
        me.phase, me.phase_init = 8, 0
    end
end

-- Run phase 8 (0x080BADD4): his timer runs out (16 ticks after his last bomb); then he flies off to the edge.
local function fly_off(me: Object)
    if me.phase_init == 0 then
        local left = me.timer - 1
        me.timer = int.u16(left)
        if left >= 0 then
            return
        end
        me.phase_init = 4
    end
    if fly(me) and at_edge(me) then
        me:set_action(0x18)
        return
    end
    me:set_panel_from_coordinates()
end

local RUN = { [0] = seek, [4] = bombard, [8] = fly_off }

-- Action 0x14 (0x080BACF4): his bombing run, by its phase.
local function run(me: Object)
    local phase = RUN[me.phase]
    if not phase then
        error(string.format("GyroMan's run: phase %#x reads past its table (0x080BAD08)", me.phase))
    end
    phase(me)
end

-- Action 0x18 (0x080BAE22): he vanishes and takes off his propeller; 31 ticks; then the controller moves on, and he
-- goes.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.visible = false
        me:remove_parts(PROPELLER)
    end
    local left = navi_chips.countdown_on_init(me, 0x1E, true)
    if left == nil or left >= 0 then
        return
    end
    navi_chips.leave(me, (me.state :: State).controller)
end

local ACTIONS = {
    [0] = appear,
    [4] = stand,
    [8] = rise,
    [0xC] = take_off,
    [0x10] = bomb_row,
    [0x14] = run,
    [0x18] = leave,
}

-- 0x080BAAA4: on his panel, on the ground; his sprite (animation 0, his shadow), his palette by the level, facing
-- his side's way; his propeller on (its sprite stepping even while paused, but Airforce's). Airforce's flight
-- starts at the take-off.
local function init(me: Object)
    navi_chips.stand(me, SPRITE, "ground", true)
    local s = me.state :: State
    me.sprite.palette = PALETTES[s.level] or 0
    me:add_parts(PROPELLER, if s.airforce then 0 else 1)
    s.run, s.bombs = false, 0
    me:set_lifecycle("update")
    if s.airforce then
        me.action = 0xC
    end
end

gyroman.kind = define.kind {
    id = "gyroman/navi",
    pool = "actor",
    state = { controller = "object", level = "u8", run = "bool", bombs = "u8", airforce = "bool" },
    -- 0x080BAA80.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            -- 0x080BAB16: Airforce's flight stands still while the battle is dimmed.
            if not ((me.state :: State).airforce and battle.dimmed()) then
                local action = ACTIONS[me.action]
                if not action then
                    error(string.format("GyroMan action %#x reads past his table (0x080BAB34)", me.action))
                end
                action(me)
            end
        else
            me:free()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- 0x080BAE60: GyroMan for `user`, reporting to `controller` (the summon table's entry 6), of a chip of `level`.
function gyroman.summon_at(level: number): (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
    return function(user: Object, controller: Object, spec: NaviChipSpec): Object?
        local o = navi_chips.spawn(gyroman.kind, user, spec)
        if not o then
            return nil
        end
        local s = o.state :: State
        s.controller = controller
        s.level = level
        return o
    end
end

-- 0x080BAE8A: GyroMan the navi's flight for `user` (himself), along row `y` from column `x`, with the element and
-- the damage word: the same object with its fourth parameter 1, its first his user's (a player's: 0), and no
-- controller.
function gyroman.fly(user: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(gyroman.kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.airforce = true
        s.level = 0
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = user
        o.alliance, o.flip = user.alliance, user.flip
        o:set_damage_word(damage)
    end
    return o
end

-- GyroMan's (his Param1 0), GyroManSP's (3), GyroManDS's (4).
gyroman.summon = gyroman.summon_at(0)
gyroman.summon_sp = gyroman.summon_at(3)
gyroman.summon_dark = gyroman.summon_at(4)

return gyroman
