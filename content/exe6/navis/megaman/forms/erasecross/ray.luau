--!strict
-- EraseCross's charged beam (the original's attack object #0x9D,
-- `sub_80D8E10`): it appears ahead of the navi, and when its opening
-- animation ends lays a hit column (DolThdr's thunder column,
-- chips/dolthdr/column, the pulling one) on each panel from its own
-- forward, up to five. Once it shuts, the columns end two ticks later and
-- the beam five. What shuts it is its
-- ray (the original's first parameter): EraseCross's lasts 70 ticks or
-- until its navi leaves the action that fired it; its Beast form's only
-- the latter; MstrCros's EraseCross's until its navi shuts it
-- (`erase_ray.shut`).

local thunder_column = require("../../../../chips/dolthdr/column")

local erase_ray = {}

-- A ray: how far ahead of its panel's center it appears
-- (`byte_80D8EA4`), 16.16; how long it lasts at most (none: until it
-- shuts); and whether its spawner shuts it, rather than its navi leaving
-- the action it fired it in.
export type Ray = { ahead: number, ticks: number?, held: boolean? }

erase_ray.rays = {
    cross = { ahead = 0x3C_0000, ticks = 70 } :: Ray,
    beast = { ahead = 0x46_0000 } :: Ray,
    held = { ahead = 0x3C_0000, held = true } :: Ray,
}

export type State = {
    -- ExtraVars+0x60..: the columns it laid (which register there), and
    -- how many (the game ends the list with -1).
    columns: { Object? },
    count: number,
    ahead: number,
    -- Its ticks at most; 0: none.
    ticks: number,
    -- Its spawner shuts it (the original's first parameter 2), and has
    -- (its second parameter).
    held: boolean,
    shut: boolean,
    -- The action its navi fired it in.
    action: Action?,
}

local SPRITE = asset.sprite("erase-ray")

-- The phases, on its phase byte.
local OPENING, BEAMING, CLOSING = 0x0, 0x4, 0x8

-- How many columns at most. (They stand 16 pixels up, the thunder column's
-- height.)
local MAX_ZONES = 5

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
local function opening(me: Object, s: State)
    if not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    me.anim = 1
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
local function beaming(me: Object, s: State)
    local shut
    if s.held then
        shut = s.shut
    else
        shut = (me.related1 :: Object):navi_action() ~= s.action
    end
    if shut then
        me.timer = 5
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
    local s = me.state :: State
    local p = me.phase
    if p == OPENING then
        opening(me, s)
    elseif p == BEAMING then
        beaming(me, s)
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
    local s = me.state :: State
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    -- (Without a time of its own, its timer is -1: the halfword's most.)
    me.timer = if s.ticks ~= 0 then s.ticks else -1
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + int.mul32(me.facing, s.ahead), p.y, p.z)
    me:set_lifecycle("update")
    tick(me)
end

erase_ray.kind = new.kind {
    id = "erasecross/ray",
    pool = "attack",
    state = {
        columns = "object[5]",
        count = "u8",
        ahead = "i32",
        ticks = "u8",
        held = "bool",
        shut = "bool",
        action = "action",
    },
    -- `sub_80D8E10`.
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

-- `sub_80D8F98`: a beam of `owner`'s on (x, y) at height `z`, of `ray`,
-- with the damage word `damage`. A ray its navi's action shuts follows the
-- action `owner` is running. (Its X and Y are its spawner's registers
-- until its init places it.)
function erase_ray.spawn(owner: Object, x: number, y: number, element: number, z: number, ray: Ray, damage: number): Object?
    local o = battle.spawn(erase_ray.kind, Vec3.new(0, 0, z))
    if o then
        local s = o.state :: State
        s.ahead, s.ticks, s.held = ray.ahead, ray.ticks or 0, ray.held or false
        if not s.held then
            local action = owner:navi_action()
            if type(action) ~= "table" then
                error("an erase ray follows an action that is no content action")
            end
            s.action = action :: Action
        end
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- `sub_80D8FB8`: shut a beam its spawner holds.
function erase_ray.shut(o: Object)
    local s = o.state :: State
    if s.held then
        s.shut = true
    end
end

return erase_ray
