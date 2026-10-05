--!strict
-- CosmoMan in the sky (EXE5's effect object #0x86, 0x080EA1BC; EXE5's own): his sprite 95 pixels up over the enemy
-- area's middle panel, calling his meteors (./meteor) down, 30 ticks apart: three, then one for each field object
-- he took. They fall on the enemies on that panel and the four beside it: by turns while the turns come out even,
-- the last ones on the one with the most HP; on the middle panel when none is there.

local meteor = require("./meteor")

local caller = {}

export type State = {
    -- The chip's level (its Param1) and how many meteors (its Param2: the objects taken, 3 more at its start).
    level: number,
    count: number,
    -- The meteors called (+6).
    called: number,
    -- The enemies it aims at (+0x64, +0x68).
    target_count: number,
    targets: { Object? },
    -- The field objects CosmoMan took, the fourth meteor's look first (his +0x60, which it reads).
    taken: { Identity? },
}

local SPRITE = asset.sprite("cosmoman")
local SOUND_CALL = asset.sound("elmnt-man")

-- 0x080EA1E0: on its panel's center, a pixel up the field, 95 pixels up (Z's fraction is the spawner's
-- register); three meteors more than the objects taken.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, int.i32(p.y - 0x1_0000), int.i32(0x5F_0000 + bit32.band(p.z, 0xFFFF)))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 19, 19
    sprite:set_animation(19)
    sprite.palette = 0
    local s = me.state :: State
    s.count = int.u8(s.count + 3)
    s.called = 0
    me:set_lifecycle("update")
end

-- 0x080EA334: the enemies on its panel and the four beside it (in their side's order).
local function aim(me: Object, s: State)
    local n = 0
    for _, o in battle.alive_actors(1 - me.alliance) do
        local near = (o.panel_x == me.panel_x and math.abs(o.panel_y - me.panel_y) <= 1)
            or (o.panel_y == me.panel_y and math.abs(o.panel_x - me.panel_x) <= 1)
        if near then
            n += 1
            s.targets[n] = o
        end
    end
    s.target_count = n
end

-- 0x080EA3BC: where the next meteor falls.
local function next_target(me: Object, s: State): (number, number)
    local n = s.target_count
    if n == 0 then
        return me.panel_x, me.panel_y
    end
    if n == 1 then
        local o = s.targets[1] :: Object
        return o.panel_x, o.panel_y
    end
    -- By turns while they come out even; then the one with the most HP.
    if s.called < s.count - s.count % n then
        local o = s.targets[s.called % n + 1] :: Object
        return o.panel_x, o.panel_y
    end
    local best: Object? = nil
    local most = 0
    for i = 1, n do
        local o = s.targets[i] :: Object
        if o.hp > most then
            best, most = o, o.hp
        end
    end
    if not best then
        error("CosmoMan's meteors: no target has HP (0x080EA3BC reads an unset slot)")
    end
    return best.panel_x, best.panel_y
end

-- Action 0 (0x080EA260): 11 ticks; then he calls (animation 20) and aims.
local function start(me: Object)
    local s = me.state :: State
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(SOUND_CALL)
        me.timer = 0xA
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left >= 0 then
        return
    end
    me.anim = 20
    me.timer = 0x1E
    aim(me, s)
    me:set_action(4)
end

-- Action 4 (0x080EA29A): a meteor every 30 ticks, CosmoMan's own for the first three.
local function call(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    local s = me.state :: State
    local x, y = next_target(me, s)
    local look = if s.called > 2 then s.taken[s.called - 2] else nil
    meteor.spawn(me, x, y, me.element, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), look)
    s.called = int.u8(s.called + 1)
    if s.called < s.count then
        me.timer = 0x1E
    else
        me:set_action(8)
    end
end

-- Action 8 (0x080EA2DE): 61 ticks, then it goes.
local function finish(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = 0x3C
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left < 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = start, [4] = call, [8] = finish }

caller.kind = define.kind {
    id = "cosmoman/caller",
    pool = "effect",
    state = {
        level = "u8",
        count = "u8",
        called = "u8",
        target_count = "u8",
        targets = "object[4]",
        taken = "identity[8]",
    },
    -- 0x080EA1BC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            -- 0x080EA232.
            if battle.over() then
                me:set_lifecycle("destroy")
            else
                local action = ACTIONS[me.action]
                if not action then
                    error(string.format("CosmoMan's caller action %#x reads past its table (0x080EA254)", me.action))
                end
                action(me)
            end
        else
            me:free()
        end
        me:update_sprite()
    end,
}

-- 0x080EA306: CosmoMan's caller over (x, y) for `cosmoman`, with his element, level, damage word and the field
-- objects he took.
function caller.spawn(cosmoman: Object, x: number, y: number, level: number, taken: { Identity? }, count: number): Object?
    local o = battle.spawn(caller.kind, Vec3.zero)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = cosmoman.element
        o.related1 = cosmoman
        o:set_damage_word(bit32.bor(cosmoman.damage, bit32.lshift(cosmoman.stamina, 16)))
        local s = o.state :: State
        s.level = level
        s.count = count
        for i = 1, count do
            s.taken[i] = taken[i]
        end
        o.alliance, o.flip = cosmoman.alliance, cosmoman.flip
        o.run_while_dimmed = true
    end
    return o
end

return caller
