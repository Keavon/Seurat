// Adapted from https://www.derschmale.com/source/hbao/HBAOFragmentShader.hlsl
// Blog post: https://www.derschmale.com/2013/12/20/an-alternative-implementation-for-hbao-2/

[[block]] struct Camera {
	v_matrix: mat4x4<f32>;
	p_matrix: mat4x4<f32>;
	inv_v_matrix: mat4x4<f32>;
	inv_p_matrix: mat4x4<f32>;
};
[[block]] struct Data {
	// samples: array<vec4<f32>, 64>;
};

// Uniforms
[[group(0), binding(0)]] var<uniform> camera: Camera;
[[group(1), binding(0)]] var<uniform> data: Data;
[[group(1), binding(1)]] var t_noise: texture_2d<f32>;
[[group(1), binding(2)]] var s_noise: sampler;
[[group(1), binding(3)]] var t_world_space_fragment_location: texture_2d<f32>;
[[group(1), binding(4)]] var s_world_space_fragment_location: sampler;
[[group(1), binding(5)]] var t_world_space_normal: texture_2d<f32>;
[[group(1), binding(6)]] var s_world_space_normal: sampler;

// Attributes
struct VertexInput {
	[[location(0)]] position: vec3<f32>;
	[[location(1)]] uv: vec2<f32>;
	[[location(2)]] normal: vec3<f32>;
	[[location(3)]] tangent: vec3<f32>;
};

// Varyings
struct VertexOutput {
	[[builtin(position)]] position: vec4<f32>;
	[[location(0)]] uv: vec2<f32>;
};

// Vertex shader
[[stage(vertex)]]
fn main(model: VertexInput) -> VertexOutput {
	return VertexOutput(vec4<f32>(model.position, 1.), model.position.xy * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5));
}

// Unproject a value from the depth buffer to the Z value in view space.
// Multiply the result with an interpolated frustum vector to get the actual view-space coordinates
float DepthToViewZ(float depthValue)
{
	return projectionMatrix[3][2] / (depthValue - projectionMatrix[2][2]);
}

// snaps a uv coord to the nearest texel centre
float2 SnapToTexel(float2 uv, float2 maxScreenCoords)
{
	return round(uv * maxScreenCoords) * rcp(maxScreenCoords);
}

// rotates a sample direction according to the row-vectors of the rotation matrix
float2 Rotate(float2 vec, float2 rotationX, float2 rotationY)
{
	float2 rotated;
	// just so we can use dot product
	float3 expanded = float3(vec, 0.0);
	rotated.x = dot(expanded.xyz, rotationX.xyy);
	rotated.y = dot(expanded.xyz, rotationY.xyy);
	return rotated;
}

// Gets the view position for a uv coord
float3 GetViewPosition(float2 uv, float2 frustumDiff)
{
	float depth = depthTexture.SampleLevel(sourceSampler, uv, 0);
	float3 frustumVector = float3(viewFrustumVectors[3].xy + uv * frustumDiff, 1.0);
	return frustumVector * DepthToViewZ(depth);
}

// Retrieves the occlusion factor for a particular sample
// uv: the centre coordinate of the kernel
// frustumVector: The frustum vector of the sample point
// centerViewPos: The view space position of the centre point
// centerNormal: The normal of the centre point
// tangent: The tangent vector in the sampling direction at the centre point
// topOcclusion: The maximum cos(angle) found so far, will be updated when a new occlusion segment has been found
float GetSampleOcclusion(float2 uv, float3 frustumVector, float3 centerViewPos, float3 centerNormal, float3 tangent, inout float topOcclusion)
{
	// reconstruct sample's view space position based on depth buffer and interpolated frustum vector
	float sampleDepth = depthTexture.SampleLevel(sourceSampler, uv, 0);
	float3 sampleViewPos = frustumVector * DepthToViewZ(sampleDepth);

	// get occlusion factor based on candidate horizon elevation
	float3 horizonVector = sampleViewPos - centerViewPos;
	float horizonVectorLength = length(horizonVector);
	
	float occlusion;

	// If the horizon vector points away from the tangent, make an estimate
	if (dot(tangent, horizonVector) < 0.0)
		return 0.5;
	else
		occlusion = dot(centerNormal, horizonVector) / horizonVectorLength;
	
	// this adds occlusion only if angle of the horizon vector is higher than the previous highest one without branching
	float diff = max(occlusion - topOcclusion, 0.0);
	topOcclusion = max(occlusion, topOcclusion);

	// attenuate occlusion contribution using distance function 1 - (d/f)^2
	float distanceFactor = saturate(horizonVectorLength / fallOff);
	distanceFactor = 1.0 - distanceFactor * distanceFactor;
	return diff * distanceFactor;
}

// Retrieves the occlusion for a given ray
// origin: The uv coordinates of the ray origin (the AO kernel centre)
// direction: The direction of the ray
// jitter: The random jitter factor by which to offset the start position
// maxScreenCoords: The maximum screen position (the texel that corresponds with uv = 1)
// projectedRadii: The sample radius in uv space
// numStepsPerRay: The amount of samples to take along the ray
// centerViewPos: The view space position of the centre point
// centerNormal: The normal of the centre point
// frustumDiff: The difference between frustum vectors horizontally and vertically, used for frustum vector interpolation
float GetRayOcclusion(float2 origin, float2 direction, float jitter, float2 maxScreenCoords, float2 projectedRadii, uint numStepsPerRay, float3 centerViewPos, float3 centerNormal, float2 frustumDiff)
{	
	// calculate the nearest neighbour sample along the direction vector
	float2 texelSizedStep = direction * rcp(renderTargetResolution);
	direction *= projectedRadii;
	
	// gets the tangent for the current ray, this will be used to handle opposing horizon vectors
	// Tangent is corrected with respect to per-pixel normal by projecting it onto the tangent plane defined by the normal
	float3 tangent = GetViewPosition(origin + texelSizedStep, frustumDiff) - centerViewPos;	
	tangent -= dot(centerNormal, tangent) * centerNormal;
	
	// calculate uv increments per marching step, snapped to texel centres to avoid depth discontinuity artefacts
	float2 stepUV = SnapToTexel(direction.xy / (numStepsPerRay - 1), maxScreenCoords);
	
	// jitter the starting position for ray marching between the nearest neighbour and the sample step size
	float2 jitteredOffset = lerp(texelSizedStep, stepUV, jitter);	
	float2 uv = SnapToTexel(origin + jitteredOffset, maxScreenCoords);
	
	// initial frustum vector matching the starting position and its per-step increments
	float3 frustumVector = float3(viewFrustumVectors[3].xy + uv * frustumDiff, 1.0);
	float2 frustumVectorStep = stepUV * frustumDiff;

	// top occlusion keeps track of the occlusion contribution of the last found occluder.
	// set to bias value to avoid near-occluders
	float topOcclusion = bias;
	float occlusion = 0.0;

	// march!
	for (uint step = 0; step < numStepsPerRay; ++step) {
		occlusion += GetSampleOcclusion(uv, frustumVector, centerViewPos, centerNormal, tangent, topOcclusion);

		uv += stepUV;
		frustumVector.xy += frustumVectorStep.xy;
	}

	return occlusion;
}

// Fragment shader
[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let halfSampleRadius = .25;

	// The maximum screen position (the texel that corresponds with uv = 1), used to snap to texels
	// (normally, this would be passed in as a constant)
	let maxScreenCoords = vec2<f32>(textureDimensions(t_world_space_fragment_location)) - vec2<f32>(1., 1.);

	// // reconstruct view-space position from depth buffer
	// float centerDepth = depthTexture.SampleLevel(sourceSampler, pin.UV, 0);
	// float3 centerViewPos = pin.FrustumVector.xyz * DepthToViewZ(centerDepth);
	
	// // unpack normal
	// let centerNormal = normalize(normalsTexture.SampleLevel(sourceSampler, in.uv, 0).xyz - .5);
	
	let view_space_fragment_location = (camera.v_matrix * vec4<f32>(textureSample(t_world_space_fragment_location, s_world_space_fragment_location, in.uv).xyz, 1.)).xyz;
	let view_space_normal = normalize((camera.v_matrix * vec4<f32>(textureSample(t_world_space_normal, s_world_space_normal, in.uv).xyz, 0.)).xyz);

	// Get the random factors and construct the row vectors for the 2D matrix from cos(a) and -sin(a) to rotate the sample directions
	let noise_scale = vec2<f32>(textureDimensions(t_world_space_fragment_location)) / vec2<f32>(textureDimensions(t_noise));
	let noise = textureSample(t_noise, s_noise, in.uv * noise_scale).xyz;
	// float3 randomFactors = ditherTexture.SampleLevel(ditherSampler, pin.UV * ditherScale, 0);
	let rotationX = normalize(noise.xy - .5);
	let rotationY = rotationX.yx * vec2<f32>(-1.0f, 1.0f);
		
	// normally, you'd pass this in as a constant, but placing it here makes things easier to understand.
	// basically, this is just so we can use UV coords to interpolate frustum vectors
	//########let frustumDiff = vec2<f32>(viewFrustumVectors[2].x - viewFrustumVectors[3].x, viewFrustumVectors[0].y - viewFrustumVectors[3].y);
	
	// scale the sample radius perspectively according to the given view depth (becomes ellipse)
	let w = view_space_fragment_location.z * camera.p_matrix[2][3] + camera.p_matrix[3][3];
	let projectedRadii = halfSampleRadius * vec2<f32>(camera.p_matrix[1][1], camera.p_matrix[2][2]) / w; // half radius because projection ([-1, 1]) -> uv ([0, 1])
	let screenRadius = projectedRadii.x * textureDimensions(t_world_space_fragment_location).x;

	// bail out if there's nothing to march
	if (screenRadius < 1.0) {
		return 1.0;
	}

	// do not take more steps than there are pixels
	uint numStepsPerRay = min(maxStepsPerRay, screenRadius);
	
	float totalOcclusion = 0.0;

	for (uint i = 0; i < numRays; ++i) {
		float2 sampleDir = Rotate(sampleDirections[i].xy, rotationX, rotationY);
		totalOcclusion += GetRayOcclusion(pin.UV, sampleDir, noise.z, maxScreenCoords, projectedRadii, numStepsPerRay, view_space_fragment_location, view_space_normal, frustumDiff);
	}

	return 1.0 - saturate(strengthPerRay * totalOcclusion);
}
