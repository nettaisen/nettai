--!strict
-- A pillar of fire or lightning on a panel (the original's attack object
-- #0x61, `sub_80D0610`): HeatCross Beast's and ElecCross Beast's charged
-- chips', Darkness's dark flames, and some viruses'. It hits its panel
-- every tick until it hits something, for its time; once its owner's
-- action is over, only a few ticks more (the flames die down, animation
-- 1, when that many are left). Lightning paralyzes and cracks its panel.
--
-- What the original's parameters said is the spawner's `ElementPillar`:
-- its first picked flames or lightning, whether it stands back, and the
-- owner's action it lasts through (`byte_80D0794`: the Beast charged
-- chips' for 0 and 1, an owner's own action for 2 to 5; past the table it
-- reads on); the others its ticks, the ticks it is left once the owner's
-- action is over, and how far back it stands.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

export type State = {
    -- Lightning (it paralyzes and cracks its panel), or flames.
    lightning: boolean,
    -- How long it stands, and the ticks left once its owner's action is
    -- over.
    ticks: number,
    after: number,
    -- It stands back: 8 pixels ahead of its panel's center, less 7 a step.
    stands_back: boolean,
    back: number,
    -- The owner's action it lasts through: a navi's (the action it ran
    -- when it sent the pillar), or an owner object's own.
    follows_navi: boolean,
    navi_action: Action?,
    owner_state: number,
}

local element_pillar = {}

local FLAMES, LIGHTNING = asset.sprite("element-pillar-flames"), asset.sprite("element-pillar-lightning")

-- Lightning's paralysis (a status byte).
local PARALYZE = statuses.paralyze_150

-- `sub_80D0766`: once the owner's action is over, `after` more ticks; the
-- flames die down with that many left.
local function watch_owner(me: Object)
    local s = me.state :: State
    if me.action == 0 then
        local owner = me.related1 :: Object
        local over
        if s.follows_navi then
            over = owner:navi_action() ~= s.navi_action
        else
            over = owner.action ~= s.owner_state
        end
        if over then
            me.timer = s.after
            me.action = s.after
        end
    end
    if me.timer == s.after then
        me.anim = 1
    end
end

-- `sub_80D072C`: resolve the hits (a hit takes its region away); then
-- present again, or end after its time.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    c:present()
    watch_owner(me)
    field.highlight(me.panel_x, me.panel_y)
end

-- `sub_80D0634`: on its panel (not off the field), raised 2 pixels, with
-- its collision; lightning cracks the panel.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local s = me.state :: State
    local lightning = s.lightning
    local sprite = me.sprite
    sprite:load(if lightning then LIGHTNING else FLAMES)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = s.ticks
    me:set_coordinates_from_panel()
    if s.stands_back then
        local p = me.pos
        local x = p.x + me.facing * 0x8_0000 - s.back * 0x7_0000 * me.facing
        me.pos = Vec3.new(x, p.y, p.z)
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y + 0x2_0000, p.z + 0x2_0000)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, if lightning then 1 else 3)
    me.collision:set_hit_effect(if lightning then sparks.elec else sparks.fire)
    if lightning then
        me.collision.status_base = PARALYZE
    end
    me.collision:present()
    if lightning then
        field.crack(me.panel_x, me.panel_y)
    end
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D0610`.
local function update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

element_pillar.kind = define.kind {
    id = "element-pillar",
    pool = "attack",
    state = {
        lightning = "bool",
        ticks = "u8",
        after = "u8",
        stands_back = "bool",
        back = "u8",
        follows_navi = "bool",
        navi_action = "action",
        owner_state = "u8",
    },
    update = update,
}

-- `sub_80D07A0`: a pillar `pillar` from `owner` on (x, y). Its position is
-- the registers (y, the element, `z`: 0 unless given, Darkness's 6 pixels)
-- until its init places it, keeping Z.
function element_pillar.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    pillar: ElementPillar,
    damage: number,
    z: number?
): Object?
    local o = battle.spawn(element_pillar.kind, Vec3.new(y, element, z or 0))
    if o then
        local s = o.state :: State
        s.lightning = pillar.lightning == true
        s.ticks, s.after = pillar.ticks, pillar.after
        local back = pillar.back
        s.stands_back = back ~= nil
        s.back = back or 0
        local owner_state = pillar.owner_state
        if owner_state then
            s.follows_navi = false
            s.owner_state = owner_state
        else
            local a = owner:navi_action()
            if type(a) ~= "table" then
                error("an element pillar follows a navi's action that is no content action")
            end
            s.follows_navi = true
            s.navi_action = a :: Action
        end
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.alliance, o.flip = owner.alliance, owner.flip
        o:set_damage_word(damage)
        o.related1 = owner
    end
    return o
end

-- `sub_80ED94E` / `sub_80EDFEA` (the Beast charged chips'): a pillar on each
-- panel of `region` from the panel in front of `owner`, facing its way,
-- that is on the field, with the attack's element and damage word.
function element_pillar.spawn_region(owner: Object, region: Region, pillar: ElementPillar)
    local front = owner.facing
    local x0, y0 = owner.panel_x + front, owner.panel_y
    local damage = int.u32(owner.attack_damage + bit32.lshift(owner.hit_param, 16))
    for _, offset in region.panels or {} do
        local x, y = x0 + offset[1] * front, y0 + offset[2]
        if x >= 1 and x <= 6 and y >= 1 and y <= 3 then
            element_pillar.spawn(owner, x, y, owner.attack_element, pillar, damage)
        end
    end
end

return element_pillar
