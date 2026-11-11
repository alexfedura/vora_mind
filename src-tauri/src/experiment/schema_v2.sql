DROP TRIGGER experiment_design_version_typed_immutable;
CREATE TRIGGER experiment_design_version_typed_immutable
BEFORE UPDATE ON research_experiment_design_versions
WHEN EXISTS (
    SELECT 1 FROM experiment_design_version_specs s
    WHERE s.design_version_id = OLD.id
)
 AND NOT (
    OLD.id = NEW.id
    AND OLD.design_id = NEW.design_id
    AND OLD.version_no = NEW.version_no
    AND OLD.description = NEW.description
    AND OLD.protocol = NEW.protocol
    AND OLD.created_by IS NEW.created_by
    AND OLD.created_at_us = NEW.created_at_us
    AND (
        (
            OLD.lifecycle_status = 'draft'
            AND NEW.lifecycle_status IN ('review', 'withdrawn')
            AND OLD.preregistered_at_us IS NEW.preregistered_at_us
        )
        OR (
            OLD.lifecycle_status = 'review'
            AND NEW.lifecycle_status = 'withdrawn'
            AND OLD.preregistered_at_us IS NEW.preregistered_at_us
        )
        OR (
            OLD.lifecycle_status = 'review'
            AND NEW.lifecycle_status = 'preregistered'
            AND OLD.preregistered_at_us IS NULL
            AND NEW.preregistered_at_us IS NOT NULL
        )
    )
)
BEGIN
    SELECT RAISE(ABORT, 'typed experiment design versions are immutable except for lifecycle transitions');
END;
