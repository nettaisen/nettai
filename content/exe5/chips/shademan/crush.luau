--!strict
-- ShadeMan's crush (EXE5's attack object #0x34, 0x080CB4AC; EXE5's own): his figure (his sprite's animation 11) two
-- panels ahead of him, 18 pixels up, that hits the panel ahead of him at once and, 10 ticks later, the column
-- beyond it (running while dimmed, no spark), highlighting where it hits, for 60 ticks. His command picks what
-- the hits carry: paralysis without a flinch, or confusion.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local crush = {}

-- What his command gave (his +0x70: the crush's parameters).
export type Effect = "none" | "paralyze" | "confuse"

export type State = {
    effect: Effect,
    -- The region it highlights (+0x60: 1, then 0x15).
    wide: boolean,
}

local SPRITE = asset.sprite("shademan")
local SOUND = asset.sound("elec-pulse")
-- By the command (its Param1 the palette and the hit modifier's choice, its Param2 the status): 0x0000, 0x1201,
-- 0x2209.
local LOOKS: { [Effect]: { palette: number, hit_mod: number, status: Status? } } = {
    none = { palette = 0, hit_mod = 3, status = nil },
    paralyze = { palette = 1, hit_mod = 0, status = require("../../rules/status").paralyze_150 },
    confuse = { palette = 9, hit_mod = 3, status = require("../../rules/status").confuse_960 },
}

-- 0x080CB5A2: a hit on (x, y) over `region` (running while dimmed, no spark), carrying the command's status.
local function hit(me: Object, x: number, y: number, region: Region)
    local look = LOOKS[(me.state :: State).effect]
    local box = battle.hitbox(me, {
        panel_x = x,
        panel_y = y,
        element = me.element,
        z = me.pos.z,
        region = region,
        hit_effect = nil,
        target = collision.hits_navis,
        self_type = collision.piercing,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = look.hit_mod,
        status = look.status,
    })
    if box then
        box.run_while_dimmed = true
    end
end

-- `object_highlightPanelRegion`: the region's panels around (x, y), turned its way (drawn only).
local function highlight(me: Object, region: Region)
    for _, o in region.panels or {} do
        field.highlight(me.panel_x + o[1] * me.facing, me.panel_y + o[2])
    end
end

-- 0x080CB538: 60 ticks; at 50 the hit on the column ahead; the region highlighted each tick. (Its end clears a
-- hit region it never had: `object_clearCollisionRegion` through no collision data.)
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left <= 0 then
        me:set_lifecycle("destroy")
        return
    end
    local s = me.state :: State
    if me.timer == 0x32 then
        hit(me, me.panel_x + me.facing, me.panel_y, regions.wide)
        s.wide = true
    end
    highlight(me, if s.wide then regions.fan else regions.single)
end

-- 0x080CB4D0: his figure (drawn with its sprite), its palette by the command; a panel further ahead than its own;
-- the hit on its panel; 60 ticks.
local function init(me: Object)
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim, me.anim_loaded = 11, 11
    sprite:set_animation(11)
    sprite.palette = LOOKS[s.effect].palette
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = 0x3C
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.facing * 0x28_0000), p.y, p.z)
    hit(me, me.panel_x, me.panel_y, regions.single)
    s.wide = false
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

crush.kind = define.kind {
    id = "shademan/crush",
    pool = "attack",
    state = { effect = { "none", "paralyze", "confuse" }, wide = "bool" },
    -- 0x080CB4AC.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080CB57C: the crush of `shademan` on (x, y), 18 pixels up, with his damage word and his command's effect.
function crush.spawn(shademan: Object, x: number, y: number, effect: Effect): Object?
    local o = battle.spawn(crush.kind, Vec3.new(y, 0, 0x12_0000))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = 0
        o:set_damage_word(bit32.bor(shademan.damage, bit32.lshift(shademan.stamina, 16)))
        o.alliance, o.flip = shademan.alliance, shademan.flip
        o.run_while_dimmed = true
        ;(o.state :: State).effect = effect
    end
    return o
end

return crush
