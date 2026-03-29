const common = {
    requireModule: ['tsx'],
    require: [
        'tests/support/**/*.ts',
        'tests/step-definitions/**/*.steps.ts',
    ],
    format: ['progress'],
    formatOptions: { snippetInterface: 'async-await' },
};

const defaultProfile = {
    ...common,
    paths: ['features/**/*.feature'],
};

export { defaultProfile as default };

export const unit = {
    ...common,
    paths: ['features/unit/**/*.feature'],
};

export const e2e = {
    ...common,
    paths: ['features/e2e/**/*.feature'],
};
