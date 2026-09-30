--!strict
-- A sword wave (attack object #0x96, `sub_80D807C`): what SlashCross's
-- charged slashes (action 0x41) send along the row. Its kind (the first
-- parameter) is data, `data.objects.sword_waves`: its collision, look,
-- speed and reach (the panel centers it passes before it ends). It hits
-- what it meets, blinks out for 10 ticks after a hit, and ends off the
-- field. The second parameter hides it; kind 0x11 also gives a bug.

export type State = {
    -- ExtraVars+0: it has moved once (the first move doesn't count a
    -- panel center).
    started: boolean,
}

local sword_wave = {
    state = { started = "bool" },
}

-- The kind that also gives a bug, and the bug (code 0x18, argument 1).
local BUGGING, BUG, BUG_ARG = 0x11, 0x18, 1

-- The kind whose sprite also steps after each update.
local STEPPING = 0x10

-- The actions: moving, blinking out after a hit.
local MOVING, BLINKING = 0, 1

-- `sub_80D8222`: a wave of `owner`'s on (x, y) with `params` (its kind
-- first) and the damage word `damage`. (Its position is its spawner's
-- registers, of which only the Z, 0 here, survives its init.)
function sword_wave.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("sword-wave", Vec3.zero, params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = bit32.band(damage, 0xFFFF)
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

-- `sub_80D8242`: the same, running while dimmed.
function sword_wave.spawn_while_dimmed(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number
): Object?
    local o = sword_wave.spawn(owner, x, y, element, params, damage)
    if o then
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_800E6E8`: moving from `old` to `new` passed `center` (reaching it
-- counts, starting on it doesn't).
local function passed(new: number, old: number, center: number): boolean
    if new > old then
        return center > old and center <= new
    end
    return center > new and center <= old
end

-- The end: hidden, no region, destroyed next update.
local function finish(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_80D8172`: resolve its hits; blink out after a hit, or move on,
-- counting the panel centers it passes; register again.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if me.action ~= MOVING then
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            finish(me)
        else
            me.visible = not bit32.btest(me.timer, 2)
        end
    elseif c.hit_flags ~= 0 then
        c.region = 0
        me.timer = 10
        me.action = BLINKING
    else
        local center = field.center(me.panel_x, me.panel_y)
        local p = me.pos
        local old = p.x
        me.pos = Vec3.new(old + me.vel.x, p.y, p.z)
        local new = me.pos.x
        local over = false
        if not s.started then
            s.started = true
        elseif passed(new, old, center) then
            local left = me:param(4) - 1
            me:set_param(4, left)
            over = left <= 0
        end
        if over then
            finish(me)
        else
            me:set_panel_from_coordinates()
            me:update_collision_panels()
            if not field.valid(me.panel_x, me.panel_y) then
                finish(me)
            end
        end
    end
    c:present()
    if me:param(3) ~= 0 then
        me.collision:highlight()
    end
    if me:param(1) == STEPPING then
        me:update_sprite()
    end
end

-- `sub_80D80B4`: its kind's look, speed, reach and registration.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local kind = me:param(1)
    local w = data.objects.sword_waves[kind]
    if w == nil then
        error(string.format("sword wave kind %#x reads past byte_80D7F4C (sub_80D80B4)", kind))
    end
    -- (The game keeps the kind's record in the first related slot, over
    -- the spawner.)
    me.related1 = nil
    local sprite = me.sprite
    sprite:load(w.sprite)
    sprite.shadow = if w.ground_shadow then "ground" else "with_sprite"
    me.anim = w.anim
    me.anim_loaded = 0xFF
    sprite.palette = w.palette
    if me:param(2) == 0 then
        me.visible = true
    end
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_param(4, w.reach)
    me:set_panel_from_coordinates()
    me.vel = Vec3.new(int.mul32(me.facing, w.speed), me.vel.y, me.vel.z)
    me:set_param(3, if w.highlight then 1 else 0)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(w.self_type, w.target_type, w.hit_mod)
    local c = me.collision
    c.hit_effect = 0xFF
    c.region = w.region
    if w.status ~= 0 then
        c.status_base = w.status
    end
    if kind == BUGGING then
        c.bugs = BUG + BUG_ARG * 0x100
    end
    c:present()
    me:update_sprite()
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D807C`: its lifecycle; then a kind that animates restarts its
-- animation after the last frame and steps.
function sword_wave.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    local w = data.objects.sword_waves[me:param(1)]
    if w and w.animates then
        if bit32.btest(me.sprite.frame_flags, 0x80) then
            me.anim_loaded = 0xFF
        end
        me:update_sprite()
    end
end

return sword_wave
