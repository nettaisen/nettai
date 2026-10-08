--!strict
-- LaserMan's laser (attack object #0xA0, 0x080E0E0C: EXE4's own), fired from the panel ahead of him along his row:
-- shown 64 pixels on (his sheet, animation 17), it fires as that ends (animation 18) and, every 6 ticks, ten times,
-- strikes its row of six panels as its course says (region 8, no spark, a piercing hit at navis): with its damage
-- (modifier 3), with none but a NaviCust bug (the hit's bug code and argument: the navi_bug hook, rules/navicust/bugs),
-- or not at all; then it ends (animation 19) as its animation does.
--
-- Its course (Param1, LaserMan's aim: 0x080E0F4C's lists), its look's palette and color shader (0x080E0E9C):
-- 0, ahead: the hit; 1, up: the attack, rapid and charge bugs (5 to 7), then the hit; 2, down: super armor, FloatShoes,
-- AirShoes and Undershirt off (1 to 4), the back special weapon gone (0x0C, 0xFF), then the hit; 3, right: the charged
-- shot the navi's own (0x0A, 1), then the hit; 4, left: the custom level one less (0x12, but not below 2), then the
-- hit. (Course 5, two steps a strike, no spawner gives.)

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local laser = {}

local SPRITE = asset.sprite("laserman")
-- A strike with its damage, and none; a course's end.
local HIT, NONE = 0xFD, 0xFF
-- The other side's custom level, one less but not below 2, as a strike (0x080E0F18).
local CUSTOM_DOWN = 0xFE
local COURSES: { [number]: { number } } = {
    [0] = { HIT },
    [1] = { 0x05, 0x06, 0x07, HIT },
    [2] = { 0x01, 0x02, 0x03, 0x04, 0xFF0C, HIT },
    [3] = { 0x10A, HIT },
    [4] = { CUSTOM_DOWN, HIT },
}
local SHADERS: { [number]: number } = { [0] = 0, [1] = 0xB060, [2] = 0xA80A, [3] = 0, [4] = 0xB9C0 }
-- Its strikes, and their period.
local STRIKES, PERIOD = 10, 6
local FIRING, ENDING = 0x0, 0x4

type State = {
    -- Its course (Param1).
    course: number,
}

-- 0x080E1000: a strike on its row (region 8 from its panel, no spark, a piercing hit at navis at its height): with its
-- damage word and hit modifier 3, or (`word`, a bug's code and argument) of no damage carrying that bug.
local function strike(me: Object, word: number)
    local damage, hit_mod, bug, bug_arg = 0, 0, 0, 0
    if word == HIT then
        damage, hit_mod = bit32.bor(me.damage, bit32.lshift(me.stamina, 16)), 3
    else
        bug, bug_arg = bit32.band(word, 0xFF), bit32.band(bit32.rshift(word, 8), 0xFF)
    end
    local box = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = me.pos.z,
        region = regions.region_08,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.piercing,
        damage = bit32.band(damage, 0xFFFF),
        stamina = bit32.rshift(damage, 16),
        hit_mod = hit_mod,
        bug = bug,
        bug_arg = bug_arg,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- Action 0 (0x080E0ED0): as animation 17 ends, animation 18 and 6 ticks; then each 6 ticks its course's next strike;
-- after the tenth, animation 19 and on to its end.
local function firing(me: Object)
    if me.anim == 17 then
        if not bit32.btest(me.sprite.frame_flags, 0x80) then
            return
        end
        me.anim = 18
        me.timer = PERIOD
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left > 0 then
        return
    end
    local i = me.timer2
    if i == STRIKES then
        me.anim = 19
        me:set_action(ENDING)
        return
    end
    me.timer2 = int.u16(i + 1)
    local word = COURSES[(me.state :: State).course][i + 1] or NONE
    if word ~= NONE then
        if word == CUSTOM_DOWN then
            local level = battle.navi(bit32.bxor(me.alliance, 1)).custom_level
            if level > 2 then
                level -= 1
            end
            word = level * 0x100 + 0x12
        end
        strike(me, word)
    end
    me.timer = PERIOD
end

-- Action 4 (0x080E0FC8): as its animation ends, it goes.
local function ending(me: Object)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me:set_lifecycle("destroy")
    end
end

-- 0x080E0E30: on its panel's center, 64 pixels on; his sheet (08-16, no shadow), shown, in animation 17, its course's
-- palette (0 ahead and down, else 10) and color shader, facing its side's way.
local function init(me: Object)
    local s = me.state :: State
    me:set_coordinates_from_panel()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 17, 17
    sprite:set_animation(17)
    sprite.palette = if s.course == 0 or s.course == 2 then 0 else 10
    sprite:set_flip(me.alliance)
    sprite.color_shader = SHADERS[s.course]
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.facing * 0x40_0000), p.y, p.z)
    me:set_lifecycle("update")
end

laser.kind = new.kind {
    id = "laserman/laser",
    pool = "attack",
    state = { course = "u8" },
    -- 0x080E0E0C: by its state, then its sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            if me.action == FIRING then
                firing(me)
            elseif me.action == ENDING then
                ending(me)
            else
                error(string.format("LaserMan's laser's action %#x reads past its table (0x080E0EC8)", me.action))
            end
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080E0FDA: a laser on (x, y) of LaserMan's side, of no element, with his damage word, on `course`, running while
-- dimmed. (Its position is the spawner's registers: the row, the element 0 and 0.)
function laser.spawn(laserman: Object, x: number, y: number, course: number): Object?
    local o = battle.spawn(laser.kind, Vec3.new(y, 0, 0))
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.element = 0
    o:set_damage_word(bit32.bor(laserman.damage, bit32.lshift(laserman.stamina, 16)))
    o.alliance = laserman.alliance
    o.run_while_dimmed = true
    ;(o.state :: State).course = course
    return o
end

return laser
