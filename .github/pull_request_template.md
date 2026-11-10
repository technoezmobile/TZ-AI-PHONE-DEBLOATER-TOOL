## Description
Briefly describe the purpose of this pull request and the problems it solves.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Catalog update (addition or refinement of debloat package ratings)
- [ ] Documentation update

## Verification & Testing
Describe the tests you ran to verify your changes:
- [ ] `npm test` passes
- [ ] `npm run build` succeeds with 0 TypeScript errors
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes (81+ tests)
- [ ] Tested on hardware / mock ADB environment

## Checklist
- [ ] My code adheres to the project's coding standards and Unslop UI principles.
- [ ] I have maintained the zero-telemetry guarantee.
- [ ] I have updated relevant documentation in `docs/` if appropriate.
- [ ] My changes do not weaken backend safety protections for launchers, keyboards, or critical services.

