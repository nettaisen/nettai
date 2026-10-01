--!strict
-- A sand hole (the original's actor object #0x1C, `sub_80BC87C`): where
-- SandWrm's worm (worm.luau) dives out of and into the ground. It opens,
-- stays open (its sprite running only while its navi can act), and closes
-- when told to (`hole.close`) or its navi is deleted. It shows only while
-- its navi does, with the worm's palette.

local hole = {}

export type State = {
    -- The worm it is for (ExtraVars).
    worm: Object?,
}

local SPRITE = asset.sprite("sand-hole")

-- The actions.
local OPEN, WAIT, CLOSE = 0, 4, 8

-- What holds its navi still (and the hole's sprite with it).
local HELD: { StatusFlag } = { "bubbled", "drag", "frozen", "paralyzed", "flinching" }

-- `sub_80BC9BE`: close `h`, if there is one and it isn't closing.
function hole.close(h: Object?)
    if h == nil or h.action == CLOSE then
        return
    end
    h.anim = 2
    h.action = CLOSE
end

-- `sub_80BC9D2`: step the sprite, unless paused, dimmed, or its navi is
-- held (bubbled, dragged, frozen, paralyzed or flinching); whether it did.
local function step(me: Object): boolean
    if battle.paused() then
        return false
    end
    local navi = me.related1 :: Object
    for _, flag in HELD do
        if navi:status(flag) then
            return false
        end
    end
    if battle.dimmed() then
        return false
    end
    me:update_sprite_even_paused()
    return true
end

-- Close, its navi gone.
local function close_now(me: Object)
    me.anim = 2
    me.action = CLOSE
    me.phase = 0
end

-- `sub_80BC920`: open, then wait.
local function open(me: Object)
    local navi = me.related1 :: Object
    if navi.hp == 0 then
        return close_now(me)
    end
    if step(me) and bit32.btest(me.sprite.frame_flags, 0x80) then
        me.anim = 1
        me.action = WAIT
        me.phase = 0
    end
end

-- `sub_80BC958`.
local function wait(me: Object)
    local navi = me.related1 :: Object
    if navi.hp == 0 then
        return close_now(me)
    end
    step(me)
end

-- `sub_80BC976`: close, then gone.
local function close(me: Object)
    if battle.paused() or battle.dimmed() then
        return
    end
    me:update_sprite_even_paused()
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80BC8EC`: the action, then shown only while its navi is.
local function tick(me: Object)
    local a = me.action
    if a == OPEN then
        open(me)
    elseif a == WAIT then
        wait(me)
    elseif a == CLOSE then
        close(me)
    else
        error(string.format("sand hole action %#x reads past its table", a))
    end
    local navi = me.related1 :: Object
    me.visible = navi.visible
end

-- `sub_80BC8A8`.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    local worm = (me.state :: State).worm :: Object
    sprite.palette = worm.sprite.palette
    sprite:set_animation(0)
    sprite:step()
    me:set_coordinates_from_panel()
    me:set_lifecycle("update")
    tick(me)
end

hole.kind = define.kind {
    id = "sandwrm/hole",
    pool = "actor",
    state = { worm = "object" },
    -- `sub_80BC87C`: gone at once once the battle is over.
    update = function(me: Object)
        if battle.over() then
            me:set_lifecycle("destroy")
        end
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
        end
    end,
}

-- `sub_80BC998`: a hole on (x, y) for `worm`, of `navi`'s. (It spawns at
-- the registers' values, which its init replaces.)
function hole.spawn(navi: Object, x: number, y: number, worm: Object): Object?
    local o = battle.spawn(hole.kind, Vec3.new(x, y, 0))
    if o then
        o.related1 = navi
        o.panel_x, o.panel_y = x, y
        o.alliance, o.flip = navi.alliance, navi.flip;
        (o.state :: State).worm = worm
    end
    return o
end

return hole
