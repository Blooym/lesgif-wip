import type {} from "@atcute/lexicons";
import * as v from "@atcute/lexicons/validations";

const _profileViewSchema = /*#__PURE__*/ v.object({
  $type: /*#__PURE__*/ v.optional(
    /*#__PURE__*/ v.literal("net.gifdex.actor.defs#profileView"),
  ),
  avatar: /*#__PURE__*/ v.optional(/*#__PURE__*/ v.genericUriString()),
  did: /*#__PURE__*/ v.didString(),
  /**
   * @maxGraphemes 64
   */
  displayName: /*#__PURE__*/ v.optional(
    /*#__PURE__*/ v.constrain(/*#__PURE__*/ v.string(), [
      /*#__PURE__*/ v.stringGraphemes(0, 64),
    ]),
  ),
  handle: /*#__PURE__*/ v.optional(/*#__PURE__*/ v.handleString()),
  postCount: /*#__PURE__*/ v.integer(),
  /**
   * @maxGraphemes 20
   */
  pronouns: /*#__PURE__*/ v.optional(
    /*#__PURE__*/ v.constrain(/*#__PURE__*/ v.string(), [
      /*#__PURE__*/ v.stringGraphemes(0, 20),
    ]),
  ),
});
const _profileViewBasicSchema = /*#__PURE__*/ v.object({
  $type: /*#__PURE__*/ v.optional(
    /*#__PURE__*/ v.literal("net.gifdex.actor.defs#profileViewBasic"),
  ),
  avatar: /*#__PURE__*/ v.optional(/*#__PURE__*/ v.genericUriString()),
  did: /*#__PURE__*/ v.didString(),
  /**
   * @maxGraphemes 64
   */
  displayName: /*#__PURE__*/ v.optional(
    /*#__PURE__*/ v.constrain(/*#__PURE__*/ v.string(), [
      /*#__PURE__*/ v.stringGraphemes(0, 64),
    ]),
  ),
  handle: /*#__PURE__*/ v.optional(/*#__PURE__*/ v.handleString()),
});
const _profileViewSearchSchema = /*#__PURE__*/ v.object({
  $type: /*#__PURE__*/ v.optional(
    /*#__PURE__*/ v.literal("net.gifdex.actor.defs#profileViewSearch"),
  ),
  avatar: /*#__PURE__*/ v.optional(/*#__PURE__*/ v.genericUriString()),
  did: /*#__PURE__*/ v.didString(),
  /**
   * @maxGraphemes 64
   */
  displayName: /*#__PURE__*/ v.optional(
    /*#__PURE__*/ v.constrain(/*#__PURE__*/ v.string(), [
      /*#__PURE__*/ v.stringGraphemes(0, 64),
    ]),
  ),
  handle: /*#__PURE__*/ v.optional(/*#__PURE__*/ v.handleString()),
});

type profileView$schematype = typeof _profileViewSchema;
type profileViewBasic$schematype = typeof _profileViewBasicSchema;
type profileViewSearch$schematype = typeof _profileViewSearchSchema;

export interface profileViewSchema extends profileView$schematype {}
export interface profileViewBasicSchema extends profileViewBasic$schematype {}
export interface profileViewSearchSchema extends profileViewSearch$schematype {}

export const profileViewSchema = _profileViewSchema as profileViewSchema;
export const profileViewBasicSchema =
  _profileViewBasicSchema as profileViewBasicSchema;
export const profileViewSearchSchema =
  _profileViewSearchSchema as profileViewSearchSchema;

export interface ProfileView extends v.InferInput<typeof profileViewSchema> {}
export interface ProfileViewBasic extends v.InferInput<
  typeof profileViewBasicSchema
> {}
export interface ProfileViewSearch extends v.InferInput<
  typeof profileViewSearchSchema
> {}
