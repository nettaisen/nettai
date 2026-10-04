--!strict
-- NumberMan's face (BN5's actor object #0x5C, 0x080C412E; BN5's own, NumberMan's alone: its spawner's only caller is
-- his, with its Param0 0 and Param2 1): a second sprite (10-06) on him, following his position, visibility, color
-- shader, alpha and palette. It waits for every navi to be in (running while paused till then); then it is drawn
-- while he stands (his animation 0), restarting its animation each time he comes back to it, and hidden otherwise.
-- He lets it go when he leaves.
--
-- Not reached: its Param0 2 and 3 (0x080C4280, shown while its owner's +0x60 object waits) and the sprites past its
-- table's first row (0x080C412C).

local overlay = {}

export type State = {
    -- It was hidden while its owner moved (+0x0C): its animation restarts when he stands again.
    hidden: boolean,
}

local SPRITE = asset.sprite("numberman-face")

-- 0x080C419C: on its owner, wearing his look; then its action.
local function follow(me: Object)
    local owner = me.related1 :: Object
    me.pos = owner.pos
    me.visible = owner.visible
    local look, worn = me.sprite, owner.sprite
    look.color_shader = worn.color_shader
    look.palette = worn.palette
    look.alpha = worn.alpha
    local s = me.state :: State
    if me.action == 0 then
        -- 0x080C41F4: every navi in, it stops running while paused.
        if battle.navis_in() then
            me.run_while_paused = false
            me:set_action(4)
        end
        return
    end
    if me.action ~= 4 then
        error(string.format("NumberMan's face: action %#x reads past its table (0x080C41EC)", me.action))
    end
    -- 0x080C4258: hidden while he moves; its animation from its start when he stands again.
    if owner.anim ~= 0 then
        me.visible = false
        s.hidden = true
    elseif s.hidden then
        s.hidden = false
        me.anim_loaded = 0xFF
    end
    me:update_sprite()
end

-- 0x080C414C: its sprite (no shadow) at animation 0, a frame on, facing its owner's way, in his palette; then its
-- first update.
local function init(me: Object)
    local owner = me.related1 :: Object
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    sprite:step()
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    sprite.palette = owner.sprite.palette
    me:set_lifecycle("update")
    local s = me.state :: State
    s.hidden = false
    follow(me)
end

overlay.kind = define.kind {
    id = "numbrman/face",
    pool = "actor",
    state = { hidden = "bool" },
    -- 0x080C412E.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            follow(me)
        else
            me:free()
        end
    end,
}

-- 0x080C42C0: NumberMan's face on `owner`, of his side and facing, running while paused.
function overlay.spawn(owner: Object): Object?
    local o = battle.spawn(overlay.kind, Vec3.zero)
    if o then
        o.related1 = owner
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_paused = true
    end
    return o
end

return overlay
