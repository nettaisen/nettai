--!strict
-- The WhitWebs' webs (attack object 0x3D, 0x080D4D30: EXE5's 0x080CCC08's code but for its constants and look), which
-- the spawners' variants 8 to 10 put out (0x080D4EC4, its row 1 to 3): one on each of the other side's panels of row
-- 1, 2 or 3 (0x0800A10E: the row's panels of the other side's, the last first), running while dimmed (its header flag
-- 0x10). A web (10-1C, turned to its side, its shadow drawn with it; its sound 0xF7) is a body with 1 HP that hits the
-- other side's (modifier 1, the wood spark) for 120 ticks, its animation 1 once animation 0 has played: it goes once it
-- caught the other side's navi, when its HP runs out (in an impact, effect row 0x0F, 8 pixels up, with sound 0x70), as
-- the battle is over, or at its end. While the battle is dimmed it only stands.
--
-- (A web of a parameter other than 0 lasts 30 ticks and sets its collision's flag 0x10, 0x08013212; the WhitWebs'
-- parameter is 0. Its spawn takes a count to keep up to date, the object's +0x4C, which the WhitWebs pass as none.)

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")

local web = {}

local SPRITE = asset.sprite("web")
local SOUND = asset.sound("web")
local GONE_SOUND = asset.sound("hit-bomb-1")
local IMPACT = new.effect { sprite = asset.sprite("impact"), anim = 0, palette = 0 }
local TICKS, HIT_MOD = 120, 1
-- The other side's navi caught, by its side (0x080D4E6C).
local CAUGHT = { [0] = 0x05800000, [1] = 0x0A800000 }
-- The other side's panels, by the user's side (0x080D4F10).
local OTHER_SIDE = { [0] = { require = 0x20, forbid = 0 }, [1] = { require = 0, forbid = 0x20 } }

local function gone(me: Object)
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- 0x080D4E74: animation 1 once animation 0 has played; gone at its end.
local function stand(me: Object)
    if me.anim == 0 and bit32.btest(me.sprite.frame_flags, 0x80) then
        me.anim = 1
    end
    local t = me.timer - 1
    me.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        gone(me)
    end
end

-- 0x080D4DFA.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        gone(me)
        return
    end
    if me:take_damage(0) == -1 then
        local p = me.pos
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x8_0000), IMPACT)
        battle.play_sound(GONE_SOUND)
        gone(me)
        return
    end
    if bit32.btest(c.hit_flags, CAUGHT[me.alliance]) then
        gone(me)
        return
    end
    if not battle.dimmed() then
        local a = me.action
        if a ~= 0 then
            error(string.format("a web's action %#x reads past its table", a))
        end
        stand(me)
    end
    c:present()
end

-- 0x080D4D68.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(me.alliance)
    me.hp, me.max_hp = 1, 1
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.own_object, collision.hit_by_other_side_or_blockers, HIT_MOD)
    me.collision:set_region(regions.single)
    me.collision:set_hit_effect(sparks.wood)
    me.timer = TICKS
    me.collision:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
end

web.kind = new.kind {
    id = "whitweb/web",
    pool = "attack",
    -- 0x080D4D30: then the sprite.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            -- 0x080D4D54 (its count, none here).
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- `object_getPanelsInRowFiltered` (0x0800A10E): row `y`'s panels meeting the filter, left to right.
local function row_of(y: number, require: number, forbid: number): { { number } }
    local list = {}
    for x = 1, 6 do
        if field.check(x, y, require, forbid) then
            table.insert(list, { x, y })
        end
    end
    return list
end

-- 0x080D4EC4, the spawners' variants 8 to 10: a web on each of the other side's panels of row `row`, the last first,
-- with the attack's element and damage word. (Its position is the registers until its init places it.)
function web.spawner(row: number): (user: Object, element: number, damage: number) -> ()
    return function(user: Object, element: number, damage: number)
        local f = OTHER_SIDE[user.alliance]
        local list = row_of(row, f.require, f.forbid)
        for i = #list, 1, -1 do
            local x, y = list[i][1], list[i][2]
            local o = battle.spawn(web.kind, Vec3.new(y, element, user.pos.z))
            if o then
                o.panel_x, o.panel_y = x, y
                o.element = element
                o:set_damage_word(damage)
                o.alliance = user.alliance
                o.run_while_dimmed = true
            end
        end
    end
end

return web
