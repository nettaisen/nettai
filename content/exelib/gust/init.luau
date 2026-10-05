--!strict
-- A gust, as EXE5 and EXE6 share it (attack object #0x49 in both: EXE6's `sub_80CD2EC`, EXE5's 0x080CEA0C, the same code
-- but for what the game's `GustLook` says): a wind that blows along its row 10 pixels a tick, pushing what it hits,
-- until it leaves the field, reaches a panel it can't cross, is hit by a wind stopper (EXE6's), or (as some spawners
-- ask) hits anything. Each game makes its kind (`gust.make_kind`).
--
-- How it blows (`GustHow`, the original's first parameter): `pulls` (bit 0) turns it back toward its side, in its
-- other palette; `unseen` (bit 7) hides it and lets it blow through any panel. A gust that pulls pushes weaker (hit
-- modifier 4 rather than 8); EXE5's, any first parameter but 0 (an unseen one too).

local gust = {}

export type State = {
    -- ExtraVars+0: it ends at its first hit (spawners set it).
    stop_on_hit: boolean,
    -- ExtraVars+4: a spawner that tracks it: the gust ends once
    -- `holder.state.gusts[slot]` is 0, and sets it to 0 when it ends. (No
    -- spawner in the game does.)
    holder: Object?,
    slot: number,
    -- How it blows (the original's first parameter's bits 0 and 7).
    pulls: boolean,
    unseen: boolean,
}

-- Its element byte (5: past the elements, what nothing is weak to).
local ELEMENT = 5

-- Its speed, 16.16 per tick.
local SPEED = 0xA_0000

-- The push its hit gives: a weak gust pushes weaker.
local PUSH, WEAK_PUSH = 8, 4
-- The palette of a gust that pulls (its first parameter times 2).
local PULL_PALETTE = 2

-- The tracking spawner's flag, and clearing it.
local function tracked(s: State): boolean
    local h = s.holder
    return h == nil or (h.state :: any).gusts[s.slot + 1] ~= 0
end

local function untrack(s: State)
    local h = s.holder
    if h then
        (h.state :: any).gusts[s.slot + 1] = 0
    end
end

-- The end: no region, hidden, destroyed next update.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- A game's gust kind, keyed `id`, of `look`.
function gust.make_kind(id: string, look: GustLook): Kind
    -- (EXE6's `sub_80CD414`, EXE5's 0x080CEB30): blow on; end on a stopping hit, off the field, or (unless unseen) on
    -- a panel it can't blow over (the game's table by side: the other side's panels; a side of 2, a turned navi's
    -- side shifted by its flip, reads the code after the table).
    local function blow(me: Object)
        local s = me.state :: State
        local c = me.collision
        local ends = bit32.btest(c.hit_flags, look.stopped_by)
        if not ends then
            local p = me.pos
            me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
            me:set_panel_from_coordinates()
            me:set_collision_panel()
            local x, y = me.panel_x, me.panel_y
            if not field.valid(x, y) then
                ends = true
            elseif s.unseen then
                return
            else
                local rule = look.panels[me.alliance]
                if rule == nil then
                    error(string.format("gust side %d reads past its panel table (sub_80CD414)", me.alliance))
                end
                ends = not field.check(x, y, rule[1], rule[2])
            end
        end
        if ends then
            c:set_region(nil)
            untrack(s)
            me:set_lifecycle("destroy")
        end
    end

    -- (EXE6's `sub_80CD3B0`): resolve its hits, then blow on (and register again).
    local function tick(me: Object)
        local s = me.state :: State
        local c = me.collision
        c:remove()
        if s.stop_on_hit and c.hit_flags ~= 0 then
            return finish(me)
        end
        if not tracked(s) then
            return finish(me)
        end
        if battle.over() then
            untrack(s)
            return finish(me)
        end
        local a = me.action
        if a ~= 0 then
            error(string.format("gust action %#x reads past its table (sub_80CD3B0)", a))
        end
        blow(me)
        c:present()
    end

    -- (EXE6's `sub_80CD310`): its look, its speed along the row (the way a pull turns it), its registration.
    local function init(me: Object)
        local s = me.state :: State
        if not s.unseen then
            me.visible = true
        end
        local sprite = me.sprite
        sprite:load(look.sprite)
        sprite.shadow = "with_sprite"
        me.anim = 0
        me.anim_loaded = 0
        sprite:set_animation(0)
        sprite:step()
        local turn = if s.pulls then 1 else 0
        sprite:set_flip(bit32.bxor(bit32.bxor(me.alliance, me.flip), turn))
        me.element = ELEMENT
        sprite.palette = if s.pulls then PULL_PALETTE else 0
        me:set_coordinates_from_panel()
        local p = me.pos
        -- object_getFlipDirection(side ^ turn, flip)
        local dir = 1 - 2 * bit32.bxor(bit32.bxor(me.alliance, turn), me.flip)
        me.pos = Vec3.new(p.x, p.y, 0)
        me.vel = Vec3.new(int.mul32(dir, SPEED), me.vel.y, me.vel.z)
        if not me:create_collision() then
            me:free()
            return
        end
        local weak = s.pulls or (look.weak_when_unseen and s.unseen)
        me:setup_collision(look.is, look.hits, if weak then WEAK_PUSH else PUSH)
        me.collision.self_damage = 0
        me.collision:present()
        me.lifecycle = "update"
        tick(me)
    end

    return define.kind {
        id = id,
        pool = "attack",
        state = { stop_on_hit = "bool", holder = "object", slot = "u8", pulls = "bool", unseen = "bool" },
        -- (EXE6's `sub_80CD2EC`.)
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
end

-- (EXE6's `sub_80CD488`): a gust of `kind` on (x, y) for side `alliance` and flip `flip` (the spawner's halfword),
-- blowing as `how` says, with the damage word `damage`, and the spawner that tracks it by `holder.state.gusts[slot]`
-- (which becomes 1), if any. Its position is the spawner's registers, which its init overwrites.
function gust.spawn(
    kind: Kind,
    x: number,
    y: number,
    alliance: number,
    flip: number,
    how: GustHow,
    damage: number,
    holder: Object?,
    slot: number
): Object?
    local o = battle.spawn(kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance, o.flip = alliance, flip
    o:set_damage_word(damage)
    local s = o.state :: State
    s.holder = holder
    s.slot = slot
    s.pulls = how.pulls == true
    s.unseen = how.unseen == true
    if holder then
        (holder.state :: any).gusts[slot + 1] = 1
    end
    return o
end

-- (EXE6's `sub_80CD4AC`, EXE5's 0x080CEBC8: the instant effect TenguCross's B+Back, EXE6's patch card dustman's and EXE5's
-- patch cards' Vacuum have): a gust of `kind` in each row, from column 6 when the user faces right (1 when it faces
-- left), pulling. Their side is the user's shifted by its flip (the game's `lsl`), their flip 0; no damage, nothing
-- tracks them; then `sound`.
function gust.blow_rows(kind: Kind, user: Object, sound: Sound)
    local facing = bit32.bxor(user.alliance, user.flip)
    local x = 5 * bit32.bxor(facing, 1) + 1
    local flip = user.flip
    local word = if flip < 32 then bit32.lshift(user.alliance, flip) else 0
    local side, side_flip = bit32.band(word, 0xFF), bit32.band(bit32.rshift(word, 8), 0xFF)
    for y = 1, 3 do
        gust.spawn(kind, x, y, side, side_flip, { pulls = true }, 0, nil, 0)
    end
    battle.play_sound(sound)
end

return gust
