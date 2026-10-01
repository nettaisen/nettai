--!strict
-- EraseCross's charged beam (attack object #0x9D, `sub_80D8E10`): it
-- appears ahead of the navi, and when its opening animation ends lays a
-- hit column (objects/thunder-column, kind 1) on each panel from its own forward,
-- up to five. Once its navi leaves the charged shot's action (0x45) it
-- shuts: the zones end two ticks later and the beam five. Its first
-- parameter: 0 lasts 70 ticks; 1 (the Beast form's) until it shuts; 2
-- shuts when its second parameter is set.

local thunder_column = require("../thunder-column/thunder_column")

export type State = {
    -- ExtraVars+0x60..: the columns it laid (which register there), and
    -- how many (the game ends the list with -1).
    columns: { Object? },
    count: number,
}

local erase_ray = {
    state = { columns = "object[5]", count = "u8" },
}

local SPRITE = "10-4c"

-- How far ahead of its panel's center it appears (`byte_80D8EA4`, by the
-- first parameter), 16.16.
local AHEAD = { [0] = 0x3C_0000, [1] = 0x46_0000, [2] = 0x3C_0000 }

-- The phases, on its phase byte.
local OPENING, BEAMING, CLOSING = 0x0, 0x4, 0x8

-- Its navi's action while the beam lasts.
local CHARGED_SHOT_ACTION = 0x45

-- How many columns at most (pulls: the thunder column's second variant,
-- `byte_80D6914`'s row 1). (They stand 16 pixels up, the thunder column's
-- height.)
local MAX_ZONES = 5

-- `sub_80D8F98`: a beam of `owner`'s on (x, y) at height `z` with `params`
-- and the damage word `damage`. (Its X and Y are its spawner's registers
-- until its init places it.)
function erase_ray.spawn(owner: Object, x: number, y: number, element: number, z: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("erase-ray", Vec3.new(0, 0, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = bit32.band(damage, 0xFFFF)
        o.stamina = bit32.rshift(damage, 16)
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `sub_80D8FB8`: shut a beam of the third kind.
function erase_ray.shut(o: Object)
    if o:param(1) == 2 then
        o:set_param(2, 1)
    end
end

-- `sub_80D8F66`: a zone on (x, its row), with its timer and whether it
-- runs while dimmed.
local function lay(me: Object, x: number, slot: number)
    local damage = bit32.bor(me.damage, bit32.lshift(me.stamina, 16))
    local z = thunder_column.spawn(me, x, me.panel_y, me.element, thunder_column.pull, damage, me, slot + 1)
    if z then
        z.run_while_dimmed = me.run_while_dimmed
        z.timer = me.timer
    end
end

-- `sub_80D8EE4`: after the opening animation, the zones from its panel
-- forward (to the field's edge, five at most).
local function opening(me: Object)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me.anim = 1
    local s = me.state :: State
    local x = me.panel_x
    local n = 0
    repeat
        lay(me, x, n)
        n += 1
        x += me.facing
    until x == 0 or x > 6 or n == MAX_ZONES
    s.count = n
    me.phase = BEAMING
end

-- `sub_80D8F20`: once it shuts, the zones end in 2 ticks and it in 5;
-- with 5 ticks or fewer left it closes.
local function beaming(me: Object)
    local shut
    if me:param(1) == 2 then
        shut = me:param(2) ~= 0
    else
        local navi = me.related1 :: Object
        shut = navi.action ~= CHARGED_SHOT_ACTION
    end
    if shut then
        me.timer = 5
        local s = me.state :: State
        for i = 1, s.count do
            local z = s.columns[i]
            if z then
                z.timer = 2
            end
        end
    end
    if me.timer <= 5 then
        me.anim = 2
        me.phase = CLOSING
    end
end

-- `sub_80D8EB0`: its phase, then its timer.
local function tick(me: Object)
    local p = me.phase
    if p == OPENING then
        opening(me)
    elseif p == BEAMING then
        beaming(me)
    elseif p ~= CLOSING then
        error(string.format("erase ray phase %#x reads past its phase table (sub_80D8EC6)", p))
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80D8E34`: its look, ahead of its panel.
local function init(me: Object)
    local kind = me:param(1)
    local ahead = AHEAD[kind]
    if ahead == nil then
        error(string.format("erase ray kind %#x reads past byte_80D8EA4 (sub_80D8E34)", kind))
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.timer = if kind == 0 then 70 else -1
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + int.mul32(me.facing, ahead), p.y, p.z)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D8E10`.
function erase_ray.update(me: Object)
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

return erase_ray
